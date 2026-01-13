# 📚 T-Library

![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange?style=for-the-badge&logo=rust&logoColor=white)
![Tauri](https://img.shields.io/badge/GUI-Tauri_v1-blue?style=for-the-badge&logo=tauri&logoColor=white)
![Platform](https://img.shields.io/badge/Platform-Windows-0078D6?style=for-the-badge&logo=windows&logoColor=white)

**T-Library**, PDF ve EPUB formatlarını destekleyen, yüksek performanslı, modern arayüzlü ve kişisel verilerinizi yerel olarak saklayan yeni nesil bir masaüstü e-kitap yönetim ve okuma uygulamasıdır.

Rust'ın gücü ve Tauri'nin hafifliği ile geliştirilen bu proje; piksel piksel işlenen özel temaları, not alma özelliklerini ve geniş kütüphane yönetimini sistem kaynaklarını yormadan sunar.

![T-Library](src-tauri/icons/128x128@2x.png)

## ✨ Özellikler

* **Çift Format Desteği:** PDF ve EPUB dosyalarını sorunsuz açar, kapak resimlerini ve meta verilerini otomatik algılar.
* **Gelişmiş Tema Motoru (Rust Backend):** Sayfaları piksel düzeyinde işleyen 6 farklı okuma modu:
    * ☀️ **Gündüz:** Standart net görünüm.
    * 🌙 **Gece & Gece Kontrast:** OLED dostu tam siyah veya yumuşak gri tonlar.
    * ☕ **Sepya & Sepya Kontrast:** Göz yormayan krem ve eski kağıt tonları.
    * 🌆 **Alacakaranlık:** Mavi ışık filtreli özel mod.
* **Yüksek Performans:** `Rayon` kütüphanesi ile çok çekirdekli (multi-thread) görsel işleme ve optimize edilmiş rendering motoru.
* **Akıllı Kütüphane:** Kitaplarınızı klasör mantığıyla ekleyin, okuma ilerlemenizi (% yüzde olarak) takip edin.
* **Notlar ve Vurgulamalar:** Okurken önemli yerleri çizin, notlar alın ve bunlara yan panelden hızla erişin.
* **Modern UI:** HTML/CSS ile tasarlanmış, tamamen özelleştirilebilir şık arayüz.
* **Veri Gizliliği:** Tüm okuma verileriniz ve notlarınız yerel bir SQLite veritabanında (`library.db`) saklanır.

## 📂 Proje Mimarisi

Bu proje, Frontend (Arayüz) ve Backend (Sistem) olarak iki ana yapıdan oluşur:

```text
T-Library/
├── .gitignore          # Gereksiz dosyaların takibi dışı bırakılması
├── package.json        # Frontend bağımlılıkları
├── src/                # Frontend (HTML/JS/CSS)
│   ├── index.html      # Ana arayüz
│   └── assets/         # İkonlar ve stiller
├── src-tauri/          # Backend (Rust)
│   ├── src/
│   │   └── main.rs     # Ana mantık, PDF işleme, Veritabanı
│   ├── Cargo.toml      # Rust kütüphaneleri
│   ├── tauri.conf.json # Uygulama yapılandırması
│   └── binaries/       # Yardımcı dosyalar (pdfium.dll vb.)
└── library.db          # Yerel veritabanı (Otomatik oluşur)
```

## 🔧 Kurulum (Geliştirici Modu)

Projeyi kaynak kodundan çalıştırmak için aşağıdaki adımları izleyin.

### 1. Gereksinimler

Sisteminizde aşağıdaki araçların yüklü olması gerekmektedir:

* **Rust:** [rustup.rs](https://rustup.rs) adresinden yükleyin.
* **Node.js:** [nodejs.org](https://nodejs.org) adresinden yükleyin.
* **VS C++ Build Tools:** (Windows için gereklidir).

### 2. Projeyi İndirin

```bash
git clone [https://github.com/ykptrkan/T-Library.git](https://github.com/ykptrkan/T-Library.git)
cd T-Library
```

### 3. Bağımlılıkları Yükleyin

Frontend paketlerini yüklemek için:

```bash
npm install
```

### 4. PDFium Kurulumu (Kritik Adım!)

Uygulamanın PDF işleyebilmesi için `pdfium.dll` dosyasına ihtiyacı vardır.

1. [PDFium Releases](https://github.com/bblanchon/pdfium-binaries/releases) adresinden sisteminize uygun (genelde `win-x64`) zip dosyasını indirin.
2. İçindeki `bin/pdfium.dll` dosyasını projenin ana dizinine veya `src-tauri/binaries/` klasörüne kopyalayın.

## 🚀 Çalıştırma

Kurulum tamamlandıktan sonra uygulamayı geliştirici modunda başlatmak için:

```bash
npm run tauri dev
```

*(İlk çalıştırmada Rust bağımlılıkları derleneceği için işlem birkaç dakika sürebilir.)*

## 📦 .EXE Olarak Derleme (Windows Uygulaması Yapma)

Bu projeyi dağıtılabilir bir `.msi` veya `.exe` kurulum dosyasına dönüştürmek için:

### 1. Derleme Komutunu Çalıştırın

```bash
npm run tauri build
```

### 2. Kurulum Dosyası

Derleme işlemi bittiğinde kurulum dosyanız şu yolda hazır olacaktır:

```text
src-tauri/target/release/bundle/msi/T-Library_0.1.0_x64_en-US.msi
```

*(Not: `pdfium.dll` dosyası yapılandırmaya göre otomatik paketlenir, çalışmazsa kurulu dizine elle kopyalamanız gerekebilir.)*

## ❓ Sıkça Sorulan Sorular

**S: Uygulama açılıyor ama hemen kapanıyor?**  
**C:** Büyük ihtimalle `pdfium.dll` eksiktir. Uygulamanın kurulu olduğu klasöre (`C:\Users\Ad\AppData\Local\T-Library`) bu dosyanın kopyalandığından emin olun.

**S: PDF sayfaları bulanık görünüyor?**  
**C:** Ayarlar menüsünden Zoom seviyesini artırabilirsiniz. Uygulama vektörel render aldığı için yakınlaştırdıkça görüntü netleşir.

## ⚠️ Yasal Uyarı

Bu proje eğitim ve kişisel kullanım amaçlı geliştirilmiştir. Uygulamaya eklenen kitapların telif haklarından ve içeriklerinden tamamen kullanıcı sorumludur.

---

**Geliştirici:** Yakup "ykptrkan" TÜRKAN