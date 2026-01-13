#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use pdfium_render::prelude::*;
use std::sync::Mutex;
use tauri::State;
use std::io::Cursor;
use image::{ImageFormat, RgbaImage}; 
use image::codecs::jpeg::JpegEncoder; 
use base64::{engine::general_purpose::STANDARD, Engine as _};
use rusqlite::{params, Connection};
use serde::{Serialize, Deserialize};
use std::fs;
use std::path::Path;
use epub::doc::EpubDoc;
use rayon::prelude::*; 

// --- VERİ MODELLERİ ---
#[derive(Serialize)]
struct Book {
    id: i32,
    title: String,
    path: String,
    cover: String,
    current_page: i32,
    total_pages: i32,
    format: String,
}

#[derive(Serialize, Deserialize)]
struct Highlight {
    id: Option<i32>,
    book_id: i32,
    page_index: i32,
    rect_json: String,
}

// --- WRAPPERS ---
struct PdfiumWrapper(Pdfium);
unsafe impl Send for PdfiumWrapper {}
unsafe impl Sync for PdfiumWrapper {}

struct DocumentWrapper(PdfDocument<'static>);
unsafe impl Send for DocumentWrapper {}
unsafe impl Sync for DocumentWrapper {}

struct AppState {
    pdfium: PdfiumWrapper,
    pdf_document: Mutex<Option<DocumentWrapper>>,
    db: Mutex<Connection>,
}

// --- DB KURULUMU ---
fn init_db() -> Connection {
    let conn = Connection::open("../library.db").expect("Veritabanı oluşturulamadı");
    conn.execute(
        "CREATE TABLE IF NOT EXISTS books (
            id INTEGER PRIMARY KEY,
            title TEXT NOT NULL,
            path TEXT NOT NULL UNIQUE,
            cover TEXT,
            current_page INTEGER DEFAULT 0,
            total_pages INTEGER DEFAULT 0,
            format TEXT DEFAULT 'pdf' 
        )", [],
    ).unwrap();
    conn.execute(
        "CREATE TABLE IF NOT EXISTS highlights (
            id INTEGER PRIMARY KEY,
            book_id INTEGER,
            page_index INTEGER,
            rect_json TEXT,
            FOREIGN KEY(book_id) REFERENCES books(id)
        )", [],
    ).unwrap();
    conn
}

// --- TEMA MOTORU (DÜZELTİLDİ: OVERFLOW HATASI GİDERİLDİ) ---
fn apply_pdf_theme(image: &mut RgbaImage, theme: &str) {
    let raw_pixels: &mut [u8] = image.as_mut();

    // Rayon ile paralel işleme
    raw_pixels.par_chunks_exact_mut(4).for_each(|pixel| {
        // DÜZELTME BURADA: u16 YERİNE u32 KULLANIYORUZ
        // u16 (max 65,535) yetmiyordu, u32 (max 4 Milyar) kullanıyoruz.
        let r = pixel[0] as u32; 
        let g = pixel[1] as u32;
        let b = pixel[2] as u32;
        // pixel[3] alpha

        match theme {
            "night" => {
                // Invert
                pixel[0] = (255 - r) as u8;
                pixel[1] = (255 - g) as u8;
                pixel[2] = (255 - b) as u8;
            },
            "night_contrast" => {
                // Luminance hesabı (Artık taşma yapmaz)
                // 255 * 587 = 149,685 (u32 içine rahat sığar)
                let lum = (r * 299 + g * 587 + b * 114) / 1000;
                if lum > 128 { 
                    pixel[0] = 0; pixel[1] = 0; pixel[2] = 0;
                } else { 
                    pixel[0] = 255; pixel[1] = 255; pixel[2] = 255;
                }
            },
            "sepia" => {
                pixel[0] = ((r * 244) / 255) as u8;
                pixel[1] = ((g * 236) / 255) as u8;
                pixel[2] = ((b * 216) / 255) as u8;
            },
            "sepia_contrast" => {
                pixel[0] = ((r * 230) / 255) as u8;
                pixel[1] = ((g * 210) / 255) as u8;
                pixel[2] = ((b * 180) / 255) as u8;
            },
            "twilight" => {
                let ir = 255 - r;
                let ig = 255 - g;
                let ib = 255 - b;
                
                pixel[0] = ((ir * 216) >> 8) as u8;
                pixel[1] = ((ig * 225) >> 8) as u8;
                pixel[2] = ((ib * 243) >> 8) as u8; 
            },
            _ => {}
        }
    });
}

// --- İŞLEMCİLER ---
fn process_pdf(path: &str, pdfium: &Pdfium) -> Result<(String, i32, String), String> {
    let doc = pdfium.load_pdf_from_file(path, None).map_err(|e| e.to_string())?;
    let total_pages = doc.pages().len() as i32;
    let page = doc.pages().get(0).map_err(|_| "Kapak alınamadı")?;
    let render_config = PdfRenderConfig::new().set_target_width(300);
    let bitmap = page.render_with_config(&render_config).map_err(|e| e.to_string())?;
    
    let mut buffer = Cursor::new(Vec::new());
    bitmap.as_image().write_to(&mut buffer, ImageFormat::Jpeg).map_err(|e| e.to_string())?;
    Ok((STANDARD.encode(buffer.into_inner()), total_pages, "pdf".to_string()))
}

fn process_epub(path: &str) -> Result<(String, i32, String), String> {
    let mut doc = EpubDoc::new(path).map_err(|e| e.to_string())?;
    let total = doc.get_num_chapters() as i32;
    let cover = doc.get_cover().map(|(d, _)| STANDARD.encode(d)).unwrap_or_default();
    Ok((cover, total, "epub".to_string()))
}

fn process_book_generic(path: &str, state: &State<AppState>, conn: &Connection) -> Result<(), String> {
    let filename = Path::new(path).file_name().unwrap().to_str().unwrap().to_string();
    let extension = Path::new(path).extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
    let (cover, total, format) = match extension.as_str() {
        "pdf" => process_pdf(path, &state.pdfium.0)?,
        "epub" => process_epub(path)?,
        _ => return Err("Desteklenmeyen format".to_string()),
    };
    conn.execute(
        "INSERT OR IGNORE INTO books (title, path, cover, total_pages, format) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![filename, path, cover, total, format],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

// --- KOMUTLAR ---

#[tauri::command]
fn get_page_image(page_index: i32, scale: f32, theme: String, state: State<AppState>) -> Result<String, String> {
    let pdf_lock = state.pdf_document.lock().map_err(|_| "Kilit hatası")?;
    if let Some(doc_wrapper) = &*pdf_lock {
        let doc = &doc_wrapper.0;
        let page = doc.pages().get(page_index as u16).map_err(|_| "Sayfa bulunamadı")?;
        
        let render_config = PdfRenderConfig::new()
            .set_target_width((595.0 * scale) as i32)
            .rotate_if_landscape(PdfPageRenderRotation::Degrees90, true);
            
        let bitmap = page.render_with_config(&render_config).map_err(|e| e.to_string())?;
        
        // RGBA8'e çevir
        let mut rgba_image = bitmap.as_image().to_rgba8(); 
        
        if theme != "day" {
            apply_pdf_theme(&mut rgba_image, &theme);
        }

        let mut buffer = Vec::new();
        // Hızlı JPEG (%90 kalite)
        let mut encoder = JpegEncoder::new_with_quality(&mut buffer, 90);
        encoder.encode_image(&rgba_image).map_err(|e| e.to_string())?;
        
        Ok(format!("data:image/jpeg;base64,{}", STANDARD.encode(buffer)))
    } else {
        Err("PDF yüklü değil".to_string())
    }
}

// ... Diğer komutlar ...
#[tauri::command]
fn add_book_to_library(path: String, state: State<AppState>) -> Result<(), String> {
    let conn = state.db.lock().map_err(|_| "DB Kilitli")?;
    process_book_generic(&path, &state, &conn)
}
#[tauri::command]
fn add_folder_to_library(folder_path: String, state: State<AppState>) -> Result<i32, String> {
    let conn = state.db.lock().map_err(|_| "DB Kilitli")?;
    let paths = fs::read_dir(folder_path).map_err(|e| e.to_string())?;
    let mut count = 0;
    for path in paths {
        if let Ok(entry) = path {
            let p = entry.path();
            if p.is_file() {
                if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                    let ext = ext.to_lowercase();
                    if ext == "pdf" || ext == "epub" {
                        if let Some(str_path) = p.to_str() {
                            if process_book_generic(str_path, &state, &conn).is_ok() { count += 1; }
                        }
                    }
                }
            }
        }
    }
    Ok(count)
}
#[tauri::command]
fn get_library(state: State<AppState>) -> Result<Vec<Book>, String> {
    let conn = state.db.lock().map_err(|_| "DB Kilitli")?;
    let mut stmt = conn.prepare("SELECT id, title, path, cover, current_page, total_pages, format FROM books").unwrap();
    let books_iter = stmt.query_map([], |row| {
        Ok(Book { id: row.get(0)?, title: row.get(1)?, path: row.get(2)?, cover: row.get(3)?, current_page: row.get(4)?, total_pages: row.get(5)?, format: row.get(6).unwrap_or("pdf".to_string()) })
    }).unwrap();
    let mut books = Vec::new();
    for book in books_iter { books.push(book.unwrap()); }
    Ok(books)
}
#[tauri::command]
fn open_book_for_reading(path: String, state: State<AppState>) -> Result<i32, String> {
    let extension = Path::new(&path).extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
    if extension == "pdf" {
        let mut pdf_lock = state.pdf_document.lock().map_err(|_| "Kilit hatası")?;
        let document = state.pdfium.0.load_pdf_from_file(&path, None).map_err(|e| e.to_string())?;
        let page_count = document.pages().len();
        unsafe {
            let static_doc = std::mem::transmute::<_, PdfDocument<'static>>(document);
            *pdf_lock = Some(DocumentWrapper(static_doc));
        }
        Ok(page_count as i32)
    } else if extension == "epub" {
        let doc = EpubDoc::new(&path).map_err(|e| e.to_string())?;
        Ok(doc.get_num_chapters() as i32)
    } else { Err("Format desteklenmiyor".to_string()) }
}
#[tauri::command]
fn get_epub_chapter(path: String, chapter_index: i32) -> Result<String, String> {
    let mut doc = EpubDoc::new(&path).map_err(|e| e.to_string())?;
    doc.set_current_chapter(chapter_index as usize);
    Ok(doc.get_current_str().map(|(s, _)| s).unwrap_or_default())
}
#[tauri::command]
fn save_progress(book_id: i32, page: i32, state: State<AppState>) -> Result<(), String> {
    let conn = state.db.lock().map_err(|_| "DB Kilitli")?;
    conn.execute("UPDATE books SET current_page = ?1 WHERE id = ?2", params![page, book_id]).map_err(|e| e.to_string())?;
    Ok(())
}
#[tauri::command]
fn save_highlight(book_id: i32, page_index: i32, rect_json: String, state: State<AppState>) -> Result<(), String> {
    let conn = state.db.lock().map_err(|_| "DB Kilitli")?;
    conn.execute("INSERT INTO highlights (book_id, page_index, rect_json) VALUES (?1, ?2, ?3)", params![book_id, page_index, rect_json]).map_err(|e| e.to_string())?;
    Ok(())
}
#[tauri::command]
fn get_highlights(book_id: i32, page_index: i32, state: State<AppState>) -> Result<Vec<String>, String> {
    let conn = state.db.lock().map_err(|_| "DB Kilitli")?;
    let mut stmt = conn.prepare("SELECT rect_json FROM highlights WHERE book_id = ?1 AND page_index = ?2").unwrap();
    let rects = stmt.query_map(params![book_id, page_index], |row| row.get(0)).unwrap();
    let mut results = Vec::new();
    for r in rects { results.push(r.unwrap()); }
    Ok(results)
}
#[tauri::command]
fn get_book_highlights(book_id: i32, state: State<AppState>) -> Result<Vec<Highlight>, String> {
    let conn = state.db.lock().map_err(|_| "DB Kilitli")?;
    let mut stmt = conn.prepare("SELECT id, book_id, page_index, rect_json FROM highlights WHERE book_id = ?1 ORDER BY page_index ASC").unwrap();
    let highlight_iter = stmt.query_map(params![book_id], |row| {
        Ok(Highlight { id: row.get(0)?, book_id: row.get(1)?, page_index: row.get(2)?, rect_json: row.get(3)? })
    }).unwrap();
    let mut results = Vec::new();
    for h in highlight_iter { results.push(h.unwrap()); }
    Ok(results)
}
#[tauri::command]
fn delete_highlight(id: i32, state: State<AppState>) -> Result<(), String> {
    let conn = state.db.lock().map_err(|_| "DB Kilitli")?;
    conn.execute("DELETE FROM highlights WHERE id = ?1", params![id]).map_err(|e| e.to_string())?;
    Ok(())
}
#[tauri::command]
fn delete_book(id: i32, state: State<AppState>) -> Result<(), String> {
    let conn = state.db.lock().map_err(|_| "DB Kilitli")?;
    conn.execute("DELETE FROM highlights WHERE book_id = ?1", params![id]).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM books WHERE id = ?1", params![id]).map_err(|e| e.to_string())?;
    Ok(())
}

fn main() {
    let db = init_db();
    let pdfium = Pdfium::new(
        Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
        .or_else(|_| Pdfium::bind_to_system_library())
        .expect("PDFium DLL dosyası bulunamadı!")
    );

    tauri::Builder::default()
        .manage(AppState {
            pdf_document: Mutex::new(None),
            pdfium: PdfiumWrapper(pdfium),
            db: Mutex::new(db),
        })
        .invoke_handler(tauri::generate_handler![
            add_book_to_library, add_folder_to_library, get_library,
            open_book_for_reading, get_page_image, get_epub_chapter,
            save_progress, save_highlight, get_highlights, get_book_highlights,
            delete_highlight, delete_book
        ])
        .run(tauri::generate_context!())
        .expect("T-Reader başlatılırken hata oluştu");
}