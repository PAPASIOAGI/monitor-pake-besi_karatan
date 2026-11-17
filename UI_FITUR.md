# Dokumentasi UI dan Fitur — PC Monitor

**Versi aplikasi:** `0.1.0`  
**Tanggal pembaruan:** 15 November 2026  
**Target platform:** Windows  
**Teknologi:** Rust, `eframe`, dan `egui`

## 1. Status Penting

PC Monitor saat ini berada pada tahap **fondasi aplikasi dan UI fungsional**.

**Seluruh nilai hardware yang ditampilkan masih berasal dari mock collector, bukan pembacaan perangkat keras Windows yang sebenarnya.** Angka bergerak secara berkala untuk menguji tampilan, histori, pause, pengaturan interval, dan komunikasi worker–UI.

Dengan demikian:

- Angka CPU bukan utilisasi prosesor komputer yang sebenarnya.
- Kapasitas dan penggunaan RAM masih berupa data simulasi.
- Nama GPU, engine, VRAM, suhu, clock, daya, dan kipas masih berupa data simulasi.
- Volume, physical disk, throughput, dan kapasitas storage masih berupa data simulasi.
- Storage Analyzer belum dapat digunakan.
- Aplikasi belum memiliki collector Windows API/PDH/DXGI yang aktif.

## 2. Struktur Antarmuka

### Jendela utama

- Judul jendela: **PC Monitor**.
- Ukuran awal: **1280 × 800 piksel**.
- Ukuran minimum: **900 × 600 piksel**.
- Area utama memiliki vertical scroll ketika konten melebihi tinggi jendela.

### Sidebar kiri

Sidebar memiliki lebar 184 piksel dan dibagi menjadi:

**MONITOR**

1. Overview
2. CPU
3. Memory
4. GPU
5. Storage

**ANALYZE**

6. Disk usage

**Pengaturan**

7. Settings, ditempatkan di bagian bawah sidebar.

Halaman aktif ditandai dengan tombol seleksi. Belum tersedia mode sidebar ringkas, ikon navigasi, maupun shortcut keyboard khusus halaman.

### Toolbar atas

Toolbar menampilkan:

- Nama halaman aktif.
- Status **Starting**, **Live**, atau **Paused**.
- Nomor urut snapshot terbaru.
- Status data **Fresh**, **Stale**, atau **Unavailable** jika status tersebut diberikan snapshot.
- Dropdown interval sampling.
- Tombol **Pause/Resume**.

### Footer

Footer menampilkan:

- `Normal user`.
- `Local only · No data leaves this PC`.

Teks tersebut merupakan label antarmuka. Teks `Normal user` bukan hasil pemeriksaan privilege Windows secara dinamis.

## 3. Halaman Overview

Overview adalah halaman awal saat aplikasi dibuka.

### Ringkasan metrik

Empat area ringkasan tersedia ketika datanya ada:

- **CPU:** total utilisasi, jumlah physical cores, dan jumlah active cores.
- **Memory:** RAM digunakan/total dan persentase penggunaan.
- **GPU:** utilisasi total dan nama GPU pertama.
- **Storage:** volume pertama, persentase penggunaan, dan kapasitas kosong.

Setiap ringkasan dilengkapi progress bar horizontal.

### Grafik Activity

Satu area grafik menampilkan tiga series:

- CPU — warna accent hijau kebiruan.
- GPU — biru.
- Memory — kuning/oranye.

Fitur grafik yang sudah ada:

- Skala vertikal 0–100%.
- Garis grid dan label 0%, 25%, 50%, 75%, dan 100%.
- Legend untuk setiap series.
- Label durasi histori dalam detik.
- Fungsi hover untuk posisi grafik dan penanda nilai.
- Ring buffer dengan kapasitas terbatas sehingga histori tidak tumbuh tanpa batas.

Deskripsi durasi grafik berasal dari pengaturan `History duration`. Saat aplikasi baru dibuka, kapasitas buffer dihitung dari durasi histori dan interval awal. **Mengubah kedua pengaturan tersebut saat aplikasi berjalan belum membangun ulang buffer secara langsung.**

### Ringkasan logical processor

- Menampilkan maksimal enam logical processor pertama.
- Setiap baris memuat nomor LP, usage bar, dan persentase.

### Ringkasan GPU engine

- Menampilkan maksimal enam engine dari GPU pertama.
- Setiap baris memuat nama engine, usage bar, dan persentase.

### Kondisi awal

Sebelum snapshot pertama diterima, halaman menampilkan `Collecting first sample…`.

## 4. Halaman CPU

Informasi yang ditampilkan:

- Nama prosesor.
- Jumlah physical cores.
- Jumlah logical processors.
- Jumlah socket.
- Total utilisasi.
- Jumlah active cores.
- Current clock dan base clock jika tersedia.
- Daftar utilisasi setiap logical processor.
- Pemetaan logical processor ke physical core.

Active core memiliki tooltip yang menjelaskan ambang penggunaan berdasarkan pengaturan aplikasi.

**Batasan saat ini:**

- Mock collector memakai ambang internal tetap 5%; perubahan `Active core threshold` belum mengubah perhitungan nilai active core.
- Nama dan topologi CPU masih simulasi 8 physical cores/16 logical processors.
- Belum ada grafik khusus halaman CPU, proses teratas, atau statistik thread proses.

## 5. Halaman Memory

Informasi yang ditampilkan:

- Physical memory digunakan dan total.
- Persentase penggunaan.
- Available memory.
- Cached memory atau `N/A`.
- Committed memory digunakan/limit.
- Page file digunakan/total atau `N/A`.

Tersedia composition bar dengan tiga legend:

- **In use**.
- **Cached**.
- **Available**.

**Catatan visual:** komposisi cache digambar setelah area in-use; nilai tersebut belum dijamin saling eksklusif. Ini adalah visualisasi awal untuk data mock, bukan representasi akurat komposisi RAM Windows.

Satuan kapasitas menggunakan B, KiB, MiB, GiB, atau TiB berdasarkan besar nilai.

## 6. Halaman GPU

Informasi yang tersedia dalam model dan UI:

- Nama adapter.
- Vendor.
- Total utilisasi.
- Engine GPU dengan nama, progress bar, dan persentase.
- Dedicated VRAM digunakan/total.
- Shared memory digunakan/total.
- Temperature dalam °C jika tersedia.
- Core clock dan memory clock jika tersedia.
- Power dalam watt jika tersedia.
- Fan RPM jika tersedia.

Mock collector saat ini memberikan engine **3D**, **Video Decode**, dan **Copy**.

Jika daftar GPU kosong, UI menampilkan `No compatible GPU adapter detected.`.

**Batasan saat ini:**

- Hanya detail GPU pertama yang dirender pada panel utama.
- Tombol nama adapter belum mengubah GPU terpilih.
- Belum ada grafik engine khusus, filter engine, pemilihan engine untuk grafik, atau integrasi NVML/ADL/IGCL.
- Pengaturan Fahrenheit belum mengonversi suhu yang ditampilkan.

## 7. Halaman Storage

### Tabel Volumes

Kolom yang tersedia:

- Volume/mount point.
- Label.
- File system.
- Used.
- Free.
- Total.
- Usage dengan progress bar dan persentase.

### Tabel Physical disks

Kolom yang tersedia:

- Nomor disk.
- Model.
- Type/media type.
- Bus.
- Total kapasitas.
- Read throughput.
- Write throughput.
- Active time.

Nilai metrik opsional ditampilkan sebagai `N/A` ketika tidak tersedia.

**Batasan saat ini:**

- Data berasal dari dua volume mock dan satu physical disk mock.
- Belum ada enumerasi drive Windows.
- Belum ada pemetaan volume ke physical disk pada UI.
- Belum ada grafik throughput atau kapasitas histori.

## 8. Halaman Disk Usage

Halaman ini baru berupa **placeholder antarmuka**.

Yang sudah ditampilkan:

- Penjelasan bahwa scan membaca metadata file.
- Input path.
- Tombol **Browse** yang dinonaktifkan.
- Tombol **Start scan** yang dinonaktifkan.
- Keterangan bahwa Storage Analyzer tersedia pada tahap pengembangan berikutnya.

Yang belum tersedia:

- Folder picker.
- State input path yang tersimpan.
- Worker scanner.
- Progress dan cancel.
- Kategori file, ukuran folder, dan file terbesar.
- Tree map atau tabel hasil scan.
- Cache hasil scan.

## 9. Halaman Settings

### Sampling interval

- Pilihan cepat pada toolbar: **250 ms, 500 ms, 1000 ms, 2000 ms, dan 5000 ms**.
- Input pada Settings menerima **100–10.000 ms**.
- Perubahan langsung dikirim ke monitoring worker tanpa restart aplikasi.

### History duration

- Rentang input: **10–3600 detik**.
- Disimpan dalam konfigurasi.
- Mengubah nilainya belum langsung mengubah kapasitas buffer yang sedang berjalan.

### Active core threshold

- Rentang input: **0–100%**.
- Digunakan oleh teks tooltip pada halaman CPU.
- Belum diteruskan ke mock collector untuk perhitungan active core.

### Theme

- Tersedia **Dark** dan **Light**.
- Perubahan berlaku langsung.
- Dark menggunakan palet Graphite khusus.
- Light memakai visual bawaan `egui`, belum palet light khusus yang selengkap tema gelap.

### Temperature unit

- Pilihan **°C** dan **°F**.
- Tersimpan dalam konfigurasi.
- Belum mengubah rendering suhu GPU.

Konfigurasi disimpan saat aplikasi keluar. File yang rusak atau nilai di luar batas validasi akan diganti/dikoreksi dengan nilai default. Environment filter `RUST_LOG` dapat digunakan untuk konfigurasi logging.

## 10. Fitur Monitoring Internal

Fitur ini bekerja di belakang UI:

- Satu worker thread bernama `monitor-worker`.
- Snapshot memiliki sequence number dan timestamp.
- Channel perintah berkapasitas 8.
- Channel snapshot berkapasitas 2.
- Jika UI tertinggal, snapshot lama dapat dibuang agar UI menerima data terbaru.
- Error collector dicatat melalui `tracing`.
- Tombol Pause menghentikan pengambilan snapshot baru tanpa menghapus data terakhir dan histori.
- Resume melanjutkan pengambilan snapshot.
- Aplikasi meminta repaint UI sekitar setiap 100 ms.
- Saat keluar, worker menerima perintah shutdown dan di-join.
- Konfigurasi disimpan saat proses keluar normal melalui lifecycle aplikasi.

## 11. Tampilan dan Gaya Visual

Tema dark menggunakan warna:

| Elemen | Warna |
|---|---|
| Window | `#161719` |
| Sidebar | `#1B1D20` |
| Surface | `#202226` |
| Surface hover | `#292C31` |
| Border | `#34383E` |
| Primary text | `#F0F1F2` |
| Secondary text | `#A5ABB3` |
| Muted text | `#737B85` |
| Accent | `#62B6A4` |
| Warning | `#D6A64F` |
| Critical, cadangan | `#DF6B67` |
| Info | `#72A7D3` |

Karakter visual:

- Border halus dan sudut sekitar 5 piksel.
- Tidak menggunakan gradasi atau glow.
- Angka detail menggunakan monospace pada area tertentu.
- Empty/loading state sederhana.
- Belum ada ikon khusus, animasi transisi, alert notification, maupun pengaturan UI scale.

## 12. Ringkasan Status Implementasi

| Area | Status aktual |
|---|---|
| Shell desktop dan navigasi | Berfungsi |
| Dark/Light theme | Berfungsi; light masih visual dasar |
| Mock data berkala | Berfungsi |
| Pause/Resume sampling | Berfungsi |
| Perubahan interval saat runtime | Berfungsi |
| Grafik CPU/RAM/GPU | Berfungsi dengan mock data |
| Detail CPU/RAM/GPU/storage | UI berfungsi dengan mock data |
| Penyimpanan konfigurasi | Berfungsi saat aplikasi keluar normal |
| Collector hardware Windows | **Belum diimplementasikan** |
| Multi-GPU switching | **Belum berfungsi** |
| Storage Analyzer | **Placeholder** |
| S.M.A.R.T. dan kesehatan disk | Belum diimplementasikan |
| Alert dan threshold warning | Belum diimplementasikan |
| Proses dengan penggunaan resource terbesar | Belum diimplementasikan |
| Histori persisten/SQLite/ekspor CSV | Belum diimplementasikan |
| Installer Windows | Belum diimplementasikan |

Dokumen ini menjelaskan keadaan source code saat tanggal pembaruan di atas, bukan target keseluruhan yang ada di `PRD.md`.
