# Product Requirements Document (PRD) — Rust PC Monitor

## 1. Ringkasan Produk

**Nama sementara:** Rust PC Monitor  
**Platform awal:** Windows 10/11 64-bit  
**Bahasa utama:** Rust stable  
**UI:** `egui` + `eframe`

Rust PC Monitor adalah aplikasi desktop untuk memantau kondisi perangkat keras PC secara real-time. Aplikasi menampilkan penggunaan CPU, RAM, GPU per engine, kapasitas dan aktivitas storage, serta menyediakan analisis isi storage berdasarkan ukuran folder dan jenis file.

Aplikasi harus tetap dapat digunakan ketika sebagian sensor, driver vendor, atau informasi S.M.A.R.T. tidak tersedia. Ketidaktersediaan data ditampilkan sebagai `N/A`, bukan dianggap sebagai nilai nol.

## 2. Tujuan

1. Memberikan gambaran kondisi PC secara cepat melalui satu dashboard.
2. Menyediakan metrik CPU dan GPU yang lebih rinci daripada sekadar persentase total.
3. Membantu pengguna memahami kapasitas, jenis, aktivitas, dan isi storage.
4. Menjalankan monitoring dengan overhead rendah tanpa membuat UI tidak responsif.
5. Menyediakan arsitektur modular agar dukungan sensor dan platform dapat ditambahkan.

## 3. Sasaran Pengguna

- Pengguna PC yang ingin memantau performa perangkat.
- Gamer dan kreator yang perlu memantau GPU 3D, encode, decode, RAM, dan VRAM.
- Teknisi atau power user yang membutuhkan informasi perangkat dan storage yang lebih rinci.

## 4. Ruang Lingkup

### 4.1 Termasuk dalam Produk

- Dashboard dengan grafik real-time.
- Monitoring CPU total dan per logical processor.
- Informasi physical core, logical processor, dan perkiraan jumlah core aktif.
- Monitoring physical memory, committed memory, dan page file.
- Monitoring multi-GPU, VRAM, dan engine GPU.
- Informasi perangkat, volume, kapasitas, filesystem, dan aktivitas storage.
- Pemindaian isi storage berdasarkan permintaan.
- Pengaturan interval sampling, durasi grafik, dan satuan.
- Penanganan sensor atau API yang tidak tersedia.

### 4.2 Tidak Termasuk dalam MVP

- Remote monitoring melalui jaringan.
- Akun pengguna, cloud sync, dan telemetri ke server.
- Kontrol fan, overclocking, undervolting, atau perubahan konfigurasi hardware.
- Benchmark dan stress test.
- Dukungan Linux/macOS.
- Riwayat berbasis database jangka panjang.
- Jumlah seluruh thread proses secara real-time melalui ETW.

Fitur di atas dapat ditambahkan setelah fungsi utama stabil.

## 5. Fitur dan Kebutuhan Fungsional

### 5.1 Dashboard

**Prioritas: P0 / MVP**

Dashboard menampilkan:

- Utilisasi CPU total.
- RAM digunakan, tersedia, dan kapasitas total.
- Utilisasi setiap GPU serta ringkasan engine utama.
- VRAM digunakan dan total jika tersedia.
- Kapasitas storage per volume.
- Suhu CPU/GPU jika tersedia dari sumber yang didukung.
- Grafik histori dalam memori untuk CPU, RAM, GPU, dan disk.
- Status setiap sumber data: aktif, sebagian tersedia, atau gagal.

Pengguna dapat memilih interval pembaruan 500 ms, 1 detik, 2 detik, atau 5 detik. UI dapat diperbarui lebih lambat daripada sampling internal jika diperlukan untuk mengurangi overhead.

### 5.2 Monitoring CPU

**Prioritas: P0 / MVP**

Data wajib:

- Nama prosesor.
- Jumlah socket, physical core, dan logical processor.
- Utilisasi total dalam persen.
- Utilisasi setiap logical processor.
- Pengelompokan logical processor berdasarkan physical core.

Data tambahan:

- Perkiraan jumlah core aktif berdasarkan ambang penggunaan.
- Base clock dan current clock jika tersedia.
- Proses dengan utilisasi CPU terbesar pada tahap lanjutan.

**Definisi core aktif:** physical core yang memiliki minimal satu logical processor dengan utilisasi di atas ambang, default 5%, selama interval sampling. Angka ini adalah estimasi berbasis utilisasi, bukan jumlah core yang benar-benar aktif secara internal.

**Catatan terminologi:** istilah “thread CPU” pada UI berarti logical processor. Istilah ini tidak merujuk pada jumlah thread milik aplikasi yang berjalan.

Sumber utama:

- `GetLogicalProcessorInformationEx` untuk topologi.
- Windows PDH/Performance Counters untuk utilisasi.

### 5.3 Monitoring RAM

**Prioritas: P0 / MVP**

- Total physical memory.
- Physical memory digunakan dan tersedia.
- Persentase penggunaan RAM.
- Committed memory dan commit limit.
- Page file/swap digunakan dan total jika tersedia.
- Cached memory jika tersedia.

Satuan default menggunakan GiB/MiB dengan label yang jelas. Nilai tidak boleh negatif, dan utilisasi harus dibatasi pada rentang 0–100% setelah validasi.

Sumber utama: `GlobalMemoryStatusEx` dan Windows Performance Counters.

### 5.4 Monitoring GPU

**Prioritas: P0 untuk engine dan memori; P1 untuk sensor vendor**

Setiap adapter ditampilkan terpisah, termasuk integrated GPU dan discrete GPU.

Data adapter:

- Nama GPU.
- Vendor dan adapter ID jika tersedia.
- Dedicated VRAM total dan digunakan.
- Shared GPU memory total dan digunakan.
- Utilisasi GPU gabungan dengan definisi yang dijelaskan pada UI.

Data engine yang harus didukung ketika tersedia:

- 3D
- Compute
- Copy
- Video Decode
- Video Encode
- Video Processing
- Engine lain yang diberikan driver

Sensor opsional:

- Temperatur.
- Core clock dan memory clock.
- Power usage.
- Fan speed.

Implementasi sumber data:

- DXGI untuk enumerasi adapter.
- PDH `GPU Engine` untuk utilisasi engine.
- PDH `GPU Adapter Memory` dan `GPU Process Memory` untuk memori.
- NVML untuk NVIDIA, AMD ADL untuk AMD, dan Intel IGCL untuk Intel.

Aturan agregasi:

- Instance counter dipetakan menggunakan PID, adapter LUID, physical engine index, dan engine type.
- Utilisasi beberapa instance engine yang sama tidak boleh langsung dijumlahkan tanpa aturan agregasi.
- Nilai total ditampilkan menggunakan metode yang konsisten, misalnya engine tersibuk atau utilisasi 3D sebagai fallback, dengan label yang jelas.
- Engine yang tidak tersedia ditampilkan sebagai `N/A`.
- Library vendor dimuat secara dinamis; kegagalan pemuatan tidak boleh menghentikan collector lain.
- Data per proses merupakan fitur tambahan dan bukan syarat MVP.

### 5.5 Informasi dan Monitoring Storage

**Prioritas: P0 untuk kapasitas; P1 untuk identitas perangkat lengkap**

Data perangkat fisik:

- Model dan kapasitas.
- SSD/HDD jika dapat ditentukan.
- Interface seperti NVMe, SATA, atau USB jika tersedia.
- Nomor perangkat fisik Windows.
- Internal/removable jika tersedia.

Data volume dan partisi:

- Drive letter dan mount point.
- Volume label.
- Filesystem: NTFS, ReFS, exFAT, FAT32, atau lainnya.
- Kapasitas total, digunakan, dan tersedia.
- Hubungan volume dengan physical disk jika dapat dipetakan.
- Network drive ditandai terpisah dan tidak diasumsikan sebagai physical disk lokal.

Data aktivitas:

- Read/write throughput.
- Persentase active time jika tersedia.
- Read/write operations per second sebagai fitur tambahan.
- S.M.A.R.T. dan health sebagai fitur tambahan yang bergantung pada perangkat, driver, dan hak akses.

Jika jenis perangkat tidak dapat dipastikan, tampilkan `Unknown`; jangan menebak hanya berdasarkan nama model.

### 5.6 Storage Analyzer

**Prioritas: P1**

Pengguna dapat memilih drive atau folder, lalu menekan **Scan**. Pemindaian tidak berjalan otomatis atau terus-menerus.

Hasil pemindaian:

- Total ukuran dan jumlah file/folder.
- Ukuran setiap folder.
- Daftar file terbesar.
- Kategori dokumen, gambar, video, audio, aplikasi, arsip, dan lainnya.
- Visualisasi tree map atau daftar hierarkis.
- Logical size sebagai default; size on disk sebagai opsi jika diimplementasikan.

Perilaku wajib:

- Scanner berjalan di luar UI thread.
- Menampilkan progres berbasis jumlah item yang telah diperiksa.
- Dapat dibatalkan.
- Tidak mengikuti junction, symlink, atau reparse point secara default.
- Melanjutkan pemindaian ketika file tidak dapat diakses dan mencatat jumlah kesalahan.
- Tidak menghapus, memindahkan, atau mengubah file pengguna.
- Hasil scan ditandai sebagai snapshot dan dapat kedaluwarsa setelah isi disk berubah.

### 5.7 Grafik dan Riwayat

**Prioritas: P0 untuk grafik dalam memori**

- Grafik minimal menyimpan 60 detik data.
- Durasi pilihan: 30 detik, 1 menit, 5 menit, dan 15 menit.
- Buffer menggunakan kapasitas maksimum agar penggunaan memori tidak terus bertambah.
- Histori hilang setelah aplikasi ditutup pada MVP.
- Persistensi SQLite dan ekspor CSV merupakan fitur P2.

### 5.8 Pengaturan

**Prioritas: P0**

- Interval sampling.
- Durasi grafik.
- Tema terang/gelap/mengikuti sistem jika tersedia.
- Satuan suhu Celsius/Fahrenheit untuk sensor yang tersedia.
- Ambang core aktif.
- Pilihan start with Windows sebagai fitur P1.

Konfigurasi disimpan secara lokal dalam format TOML. Nilai konfigurasi yang rusak tidak boleh menyebabkan aplikasi gagal total; gunakan default dan tampilkan peringatan.

### 5.9 Alert dan Status

**Prioritas: P2**

- Peringatan RAM, storage, suhu, dan utilisasi tinggi berdasarkan ambang pengguna.
- Hysteresis atau durasi minimum untuk mencegah alert berulang.
- Tidak ada notifikasi suara secara default.

## 6. Kebutuhan Nonfungsional

### Performa

- Target overhead rata-rata aplikasi: CPU kurang dari 1–2% pada interval 1 detik, bergantung hardware.
- Target memori dasar kurang dari 100 MB, di luar cache hasil storage scan.
- UI tidak boleh terblokir oleh PDH, WMI, storage scan, atau inisialisasi driver.
- Storage scan tidak boleh membuat alokasi data tanpa batas.

### Reliabilitas

- Kegagalan satu collector tidak menghentikan collector lain.
- Error berulang diberi backoff agar log tidak dibanjiri pesan yang sama.
- Data terakhir dapat ditampilkan dengan penanda `stale` dan timestamp.
- Aplikasi dapat keluar dengan bersih serta menghentikan worker dan scanner.

### Keamanan dan Privasi

- Tidak ada pengiriman data ke server pada ruang lingkup awal.
- Tidak memerlukan akun.
- Tidak menjalankan perintah sistem yang mengubah konfigurasi.
- Aplikasi berjalan sebagai pengguna biasa; mode administrator hanya diperlukan untuk fitur tertentu jika benar-benar dibutuhkan.
- Storage analyzer hanya membaca metadata dan tidak membaca isi dokumen untuk kategorisasi.

### Usability dan Aksesibilitas

- Satuan dan definisi metrik ditampilkan jelas.
- `N/A`, `0`, dan data basi harus dibedakan.
- Warna bukan satu-satunya indikator status.
- Layout tetap terbaca pada ukuran jendela desktop umum dan display scaling Windows.

## 7. Desain Teknis Ringkas

UI `egui` berkomunikasi dengan monitoring service melalui message channel. Collector mengambil data secara berkala di worker thread dan menghasilkan `SystemSnapshot`. UI hanya membaca snapshot terbaru dan buffer histori, tidak memanggil API hardware secara langsung.

Storage analyzer merupakan pekerjaan terpisah dengan mekanisme progres dan pembatalan. Implementasi awal menggunakan thread Rust dan `crossbeam-channel`; async runtime hanya ditambahkan jika ada kebutuhan konkret.

Rincian modul, aliran data, dan konvensi implementasi terdapat pada [`STRUCTURE.md`](./STRUCTURE.md).

## 8. Prioritas dan Tahapan

| Tahap | Cakupan | Kriteria selesai utama |
|---|---|---|
| 1. Fondasi | Shell UI, konfigurasi, snapshot, worker | Aplikasi berjalan, UI responsif, shutdown bersih |
| 2. MVP | CPU, RAM, kapasitas disk, dashboard, grafik | Data sesuai toleransi dan update stabil |
| 3. GPU & storage | Engine GPU, VRAM, identitas disk, throughput | Multi-GPU dan sensor unavailable ditangani |
| 4. Storage Analyzer | Scan, kategori, file terbesar, tree map | Progres dan cancel berfungsi tanpa membekukan UI |
| 5. Lanjutan | Sensor vendor, SMART, alert, persistensi | Fallback dan pengujian hardware memadai |

## 9. Kriteria Penerimaan MVP

1. Aplikasi menampilkan CPU total dan per logical processor dengan pembaruan sesuai interval.
2. Topologi physical core/logical processor sesuai dengan informasi Windows.
3. RAM total, digunakan, tersedia, dan commit ditampilkan dengan satuan benar.
4. Setiap volume lokal menampilkan kapasitas total, digunakan, dan tersedia.
5. Dashboard menyediakan grafik minimal 60 detik dengan buffer terbatas.
6. UI tetap merespons input ketika collector lambat atau gagal.
7. Sensor yang tidak didukung menampilkan `N/A`, bukan menyebabkan panic.
8. Aplikasi dapat dibuka dan ditutup berulang kali tanpa worker tertinggal.
9. Tidak ada akses jaringan atau pengumpulan data eksternal.

## 10. Strategi Pengujian

- Unit test konversi satuan, agregasi, ring buffer, dan validasi konfigurasi.
- Fixture teks untuk instance PDH agar parser GPU dapat diuji tanpa GPU tertentu.
- Integration test scanner menggunakan direktori sementara, file semu, permission error jika dapat disimulasikan, dan junction.
- Perbandingan manual dengan Task Manager dan HWiNFO; perbedaan kecil akibat waktu sampling dapat diterima.
- Pengujian pada iGPU, dGPU, hybrid graphics, multi-drive, USB storage, network drive, dan path Unicode.
- Pengujian interval sampling dan cancel scan berulang untuk mendeteksi race atau resource leak.

## 11. Risiko dan Mitigasi

| Risiko | Mitigasi |
|---|---|
| Nama counter PDH berbeda menurut bahasa Windows | Gunakan counter ID/path yang sesuai, bukan hanya string lokal; uji pada Windows non-Inggris |
| Instance GPU sulit dipetakan dan berlebih | Parsing terstruktur, deduplikasi, fixture, dan validasi nilai |
| Sensor vendor tidak seragam | Adapter per vendor, dynamic loading, dan fallback `N/A` |
| WMI lambat atau gagal | Jalankan di worker, cache data statis, dan jangan polling WMI pada setiap frame |
| Scan storage berat | On-demand, cancellation, batch update, serta batas hasil/cache |
| Angka monitoring berbeda dari Task Manager | Samakan interval dan definisi metrik; dokumentasikan metode agregasi |

## 12. Keputusan Awal

- Fokus Windows terlebih dahulu.
- Menggunakan native Rust UI melalui `egui`, bukan web UI.
- Monitoring real-time lebih diprioritaskan daripada riwayat jangka panjang.
- Storage Analyzer dibangun setelah monitoring utama stabil.
- Informasi sensor yang tidak dapat diverifikasi ditampilkan sebagai tidak tersedia, bukan diperkirakan.
