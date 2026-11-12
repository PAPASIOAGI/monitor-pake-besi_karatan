# Rancangan Struktur Proyek — Rust PC Monitor

Dokumen ini menjadi dasar struktur source code dan pembagian tanggung jawab modul. Kebutuhan produk serta prioritas fitur berada di [`PRD.md`](./PRD.md).

## 1. Prinsip Arsitektur

1. **UI terpisah dari pengambilan data.** Widget tidak memanggil Windows API atau membaca disk secara langsung.
2. **Satu snapshot yang konsisten.** UI menerima data terstruktur melalui `SystemSnapshot`, bukan kumpulan nilai global yang dapat berubah saat dirender.
3. **Collector independen.** Error CPU, GPU, storage, atau sensor tidak menghentikan seluruh aplikasi.
4. **Data statis di-cache.** Nama CPU, topologi, model disk, dan informasi adapter tidak perlu dibaca pada setiap frame.
5. **Pekerjaan berat on-demand.** Storage scan hanya berjalan atas perintah pengguna dan dapat dibatalkan.
6. **Nilai tidak tersedia berbeda dari nol.** Gunakan `Option<T>` untuk sensor atau metrik yang tidak tersedia.
7. **Implementasi Windows terisolasi.** Kode `windows` crate berada pada modul platform agar domain dan UI tidak bergantung langsung pada API OS.

## 2. Teknologi Dasar

| Kebutuhan | Pilihan awal |
|---|---|
| Rust | Stable toolchain, edition 2024 |
| Desktop UI | `egui`, `eframe` |
| Windows API | `windows` |
| Channel | `crossbeam-channel` |
| Worker | `std::thread` untuk collector periodik |
| Konfigurasi | `serde`, `toml`, `directories` |
| Storage traversal | `jwalk` atau `walkdir`; pilih satu saat implementasi |
| Error library | `thiserror` |
| Error aplikasi | `anyhow` pada boundary aplikasi |
| Logging | `tracing`, `tracing-subscriber` |
| Test temporary directory | `tempfile` |
| Visualisasi | Widget grafik `egui`; tambah plot crate hanya jika perlu |
| Persistensi lanjutan | `rusqlite`, belum diperlukan untuk MVP |

Hindari menambahkan async runtime pada MVP jika seluruh operasi dapat ditangani oleh worker thread dan channel. Tambahkan runtime hanya ketika ada kebutuhan yang jelas.

## 3. Struktur Direktori

```text
monitor/
├── Cargo.toml
├── Cargo.lock
├── rustfmt.toml
├── README.md
├── PRD.md
├── STRUCTURE.md
├── build.rs
├── assets/
│   ├── icons/
│   └── fonts/
├── src/
│   ├── main.rs
│   ├── app.rs
│   ├── error.rs
│   ├── config.rs
│   │
│   ├── domain/
│   │   ├── mod.rs
│   │   ├── snapshot.rs
│   │   ├── cpu.rs
│   │   ├── memory.rs
│   │   ├── gpu.rs
│   │   ├── storage.rs
│   │   ├── units.rs
│   │   └── status.rs
│   │
│   ├── monitoring/
│   │   ├── mod.rs
│   │   ├── service.rs
│   │   ├── messages.rs
│   │   ├── scheduler.rs
│   │   ├── collector.rs
│   │   └── history.rs
│   │
│   ├── platform/
│   │   ├── mod.rs
│   │   └── windows/
│   │       ├── mod.rs
│   │       ├── cpu.rs
│   │       ├── memory.rs
│   │       ├── disk.rs
│   │       ├── volume.rs
│   │       ├── pdh.rs
│   │       ├── gpu.rs
│   │       └── sensors/
│   │           ├── mod.rs
│   │           ├── nvidia.rs
│   │           ├── amd.rs
│   │           └── intel.rs
│   │
│   ├── storage_analyzer/
│   │   ├── mod.rs
│   │   ├── scanner.rs
│   │   ├── model.rs
│   │   ├── categories.rs
│   │   ├── messages.rs
│   │   └── cache.rs
│   │
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── dashboard.rs
│   │   ├── cpu_page.rs
│   │   ├── memory_page.rs
│   │   ├── gpu_page.rs
│   │   ├── storage_page.rs
│   │   ├── analyzer_page.rs
│   │   ├── settings_page.rs
│   │   ├── navigation.rs
│   │   ├── theme.rs
│   │   └── widgets/
│   │       ├── mod.rs
│   │       ├── metric_card.rs
│   │       ├── time_series.rs
│   │       ├── gauge.rs
│   │       └── tree_map.rs
│   │
│   └── util/
│       ├── mod.rs
│       ├── format.rs
│       └── time.rs
│
├── tests/
│   ├── storage_scanner_tests.rs
│   ├── snapshot_tests.rs
│   └── fixtures/
│       └── pdh/
│           ├── gpu_engine_single_gpu.txt
│           ├── gpu_engine_hybrid.txt
│           └── gpu_engine_multiple_processes.txt
│
└── packaging/
    └── windows/
        ├── wix/
        └── assets/
```

File dan modul untuk fitur P1/P2 dapat dibuat ketika tahapannya dimulai. Tidak perlu membuat seluruh file kosong pada inisialisasi proyek.

## 4. Tanggung Jawab Modul

### 4.1 Entry Point dan Aplikasi

#### `src/main.rs`

- Inisialisasi logging.
- Memuat dan memvalidasi konfigurasi.
- Menyiapkan native window `eframe`.
- Menangani error startup pada level aplikasi.

#### `src/app.rs`

- Menyimpan state aplikasi dan halaman aktif.
- Memulai `MonitoringService`.
- Menerima snapshot dan event scanner secara non-blocking.
- Meminta repaint sesuai kebutuhan.
- Mengirim perintah shutdown dan menunggu worker berhenti dengan timeout.

#### `src/config.rs`

Berisi struktur konfigurasi dan nilai default, misalnya:

- `sampling_interval_ms`
- `history_duration_secs`
- `active_core_threshold_percent`
- `theme`
- `temperature_unit`

Konfigurasi divalidasi sebelum digunakan. Jangan menulis konfigurasi pada setiap frame.

### 4.2 Domain

Domain berisi tipe yang dipakai bersama dan tidak bergantung pada `eframe`, `windows`, ataupun format counter mentah.

#### `domain/snapshot.rs`

Model utama yang disarankan:

```rust
pub struct SystemSnapshot {
    pub sequence: u64,
    pub captured_at: std::time::SystemTime,
    pub cpu: CpuSnapshot,
    pub memory: MemorySnapshot,
    pub gpus: Vec<GpuSnapshot>,
    pub storage: StorageSnapshot,
    pub warnings: Vec<CollectorWarning>,
}
```

`sequence` membantu UI mengenali data baru. `captured_at` digunakan untuk menandai data basi.

#### `domain/status.rs`

Gunakan status eksplisit untuk ketersediaan data:

```rust
pub enum DataStatus {
    Fresh,
    Stale,
    Unavailable,
}
```

Nilai numerik sensor tetap menggunakan `Option<T>`. Jangan menggunakan `-1` sebagai sentinel.

#### `domain/cpu.rs`

Berisi informasi statis dan snapshot dinamis:

- Nama prosesor, socket, physical core, logical processor.
- Total utilization.
- Utilization per logical processor.
- Pemetaan logical processor ke physical core.
- Perkiraan jumlah core aktif.
- Clock sebagai `Option`.

#### `domain/memory.rs`

- Total, used, available bytes.
- Commit used/limit bytes.
- Page file used/total sebagai opsi.
- Cached bytes sebagai opsi.

#### `domain/gpu.rs`

- Identitas adapter dan LUID internal Windows bila diperlukan.
- Engine dengan tipe `GpuEngineKind` dan utilisasi.
- Dedicated/shared memory.
- Temperature, clock, power, fan sebagai `Option`.
- Status source PDH dan vendor secara terpisah.

#### `domain/storage.rs`

Pisahkan tiga konsep:

- `PhysicalDisk`: perangkat keras.
- `Volume`: drive letter/mount point dan filesystem.
- `DiskActivity`: throughput dan active time.

Satu physical disk dapat memiliki beberapa volume. Jangan menggunakan drive letter sebagai primary key perangkat fisik.

#### `domain/units.rs`

Gunakan tipe satuan yang jelas untuk nilai yang rentan tertukar, misalnya bytes, bytes per second, Celsius, MHz, watt, dan percent. Persentase divalidasi/dibatasi pada boundary collector, bukan diam-diam diubah di widget.

### 4.3 Monitoring Service

#### `monitoring/messages.rs`

Perintah UI ke service:

```text
Start
SetInterval(Duration)
RefreshStaticInfo
Shutdown
```

Event service ke UI:

```text
Snapshot(SystemSnapshot)
CollectorWarning(CollectorWarning)
Stopped
```

Channel UI harus bounded. Jika UI lambat, snapshot lama boleh dibuang dan hanya snapshot terbaru yang dipertahankan. Jangan menumpuk snapshot tanpa batas.

#### `monitoring/service.rs`

- Memiliki handle worker dan channel.
- Menjalankan koordinasi lifecycle.
- Menggabungkan hasil collector menjadi snapshot.
- Mengirim warning tanpa menghentikan service.
- Menjamin shutdown idempoten.

#### `monitoring/scheduler.rs`

- Menjadwalkan sampling berdasarkan interval.
- Menghindari burst ketika sistem baru bangun dari sleep.
- Tidak menganggap timestamp lama sebagai data baru.
- Tidak perlu polling semua data pada frekuensi sama; informasi statis hanya dibaca saat startup/refresh.

#### `monitoring/collector.rs`

Trait awal untuk keluaran yang dapat digabungkan:

```rust
pub trait Collector {
    type Output;

    fn collect(&mut self) -> Result<Self::Output, CollectorError>;
}
```

Trait ini dapat disederhanakan bila generic tidak memberi manfaat. Hindari abstraksi plugin dinamis pada MVP.

#### `monitoring/history.rs`

- Ring buffer berkapasitas tetap per metrik.
- Penyimpanan timestamp dan nilai.
- Downsampling untuk rentang panjang jika diperlukan.
- Tidak menyimpan seluruh snapshot tanpa batas.

### 4.4 Windows Platform Layer

#### `platform/windows/pdh.rs`

Wrapper aman untuk:

- Membuka/menutup query dan menghapus counter.
- Menambahkan counter total, per logical processor, GPU, dan disk.
- Mengumpulkan sampel pada waktu yang tepat.
- Membaca formatted values dan menangani status PDH seperti data belum tersedia.
- Menyediakan bentuk instance counter yang sudah diparsing ke collector GPU.

Semua handle PDH harus memiliki cleanup melalui RAII. Data counter rate memerlukan lebih dari satu sampel; sampel pertama tidak boleh dianggap valid jika API menyatakan demikian.

Nama counter dapat bergantung pada lokalisasi Windows. Implementasi harus mempertimbangkan counter ID atau mekanisme path yang tidak bergantung pada teks bahasa lokal.

#### `platform/windows/cpu.rs`

- Query topologi melalui `GetLogicalProcessorInformationEx`.
- Menyusun logical processor dan physical core.
- Membaca counter utilisasi dari wrapper PDH.
- Menghitung estimasi active core di domain/monitoring berdasarkan data mentah.

#### `platform/windows/memory.rs`

- Membaca `GlobalMemoryStatusEx`.
- Mengambil commit/cached/page file tambahan dari counter yang relevan.
- Memvalidasi relasi used + available terhadap total tanpa panic jika ada perbedaan sampling.

#### `platform/windows/gpu.rs`

Alur yang disarankan:

1. Enumerasi adapter menggunakan DXGI.
2. Baca instance counter `GPU Engine`.
3. Parse atribut seperti PID, LUID, engine index, dan engine type.
4. Deduplikasi instance yang setara.
5. Agregasikan per adapter dan engine.
6. Ambil dedicated/shared memory.
7. Gabungkan sensor vendor bila tersedia.

Hasil parsing harus dapat diuji melalui fixture tanpa memanggil Windows. Pisahkan fungsi murni `parse_gpu_engine_instance` dan agregasi dari handle PDH.

#### `platform/windows/disk.rs`

- Enumerasi physical disk.
- Query bus type, media type, model, dan kapasitas.
- Membaca aktivitas disk melalui PDH.
- Menangani data yang tidak tersedia pada USB, RAID, virtual disk, atau driver tertentu.

#### `platform/windows/volume.rs`

- Enumerasi volume dan mount point.
- Membaca label, filesystem, dan kapasitas.
- Memetakan volume ke physical disk jika memungkinkan.
- Menandai removable/network/unknown tanpa menebak.

#### `platform/windows/sensors/`

Satu adapter per vendor dengan interface keluaran yang seragam. Dynamic library:

- Dimuat sekali dengan prosedur inisialisasi yang aman.
- Tidak diasumsikan selalu tersedia.
- Tidak dicampur dengan pengambilan metrik PDH.
- Ditutup/dibersihkan sesuai dokumentasi vendor.

Urutan awal: sensor vendor belum menjadi bagian MVP. Implementasi PDH GPU tidak boleh menunggu modul ini selesai.

### 4.5 Storage Analyzer

#### `storage_analyzer/model.rs`

- `ScanId` untuk membedakan hasil dari scan yang berbeda.
- Root path, folder tree, file size, kategori, jumlah error.
- `ScanProgress` dan ringkasan hasil.
- Hindari menyimpan metadata berulang yang tidak diperlukan.

#### `storage_analyzer/messages.rs`

Perintah:

```text
StartScan { root, options }
CancelScan { scan_id }
ClearResult
```

Event:

```text
Started { scan_id }
Progress { scan_id, scanned_files, scanned_dirs, bytes }
Completed { scan_id, result }
Cancelled { scan_id }
Failed { scan_id, error }
```

UI mengabaikan event yang `scan_id`-nya bukan scan aktif.

#### `storage_analyzer/scanner.rs`

- Berjalan di worker tersendiri.
- Memeriksa cancellation secara berkala.
- Tidak mengikuti reparse point secara default.
- Mengirim progres dalam batch, bukan satu message per file.
- Mengumpulkan error tanpa berhenti pada file pertama yang tidak dapat diakses.
- Menghindari penghitungan ulang hard link jika dukungan ini ditambahkan.

#### `storage_analyzer/categories.rs`

Kategorisasi berdasarkan ekstensi menggunakan lookup statis. Ekstensi tidak dikenal masuk `Other`. Nama file tidak dikirim keluar aplikasi dan isi file tidak dibaca.

#### `storage_analyzer/cache.rs`

Cache opsional untuk hasil terakhir. Cache harus menyimpan root dan waktu scan, memiliki batas memori, dan tidak diasumsikan selalu mutakhir.

### 4.6 UI

Setiap page hanya bertugas menampilkan state dan membuat aksi pengguna.

- `dashboard.rs`: ringkasan dan grafik utama.
- `cpu_page.rs`: total, per logical processor, dan topologi.
- `memory_page.rs`: physical memory, commit, dan page file.
- `gpu_page.rs`: pemilihan adapter, engine, VRAM, dan sensor.
- `storage_page.rs`: physical disk, volume, dan aktivitas.
- `analyzer_page.rs`: pemilihan path, progres, cancel, hasil, dan tree map.
- `settings_page.rs`: konfigurasi aplikasi.
- `widgets/`: komponen presentasional yang dapat digunakan ulang.

Widget tidak boleh:

- Melakukan polling hardware.
- Membuka file atau melakukan scan disk.
- Menunggu channel.
- Mengubah data mentah menjadi seolah-olah valid.

Formatter pada `util/format.rs` menangani bytes, throughput, persentase, dan `N/A` secara konsisten.

## 5. Aliran Data

### Monitoring

```text
UI command
   │
   ▼
MonitoringService / Scheduler
   │
   ├── CPU collector ──────┐
   ├── Memory collector ───┤
   ├── GPU collector ──────┼──► SystemSnapshot ──► bounded channel ──► UI
   └── Storage collector ──┘             │
                                         ▼
                                    HistoryBuffer
```

Pada implementasi awal, collector dapat berjalan berurutan dalam satu worker jika overhead dan latensinya rendah. Pisahkan GPU/WMI yang lambat ke worker lain hanya setelah terbukti diperlukan. Pendekatan ini lebih sederhana daripada membuat satu thread per metrik sejak awal.

### Storage Analyzer

```text
User: Start Scan
      │
      ▼
Scanner worker ──► progress (batch) ──► UI
      │
      ├── Cancel signal
      └── Completed/Cancelled/Failed ──► UI
```

## 6. Aturan Concurrency dan Lifecycle

- Semua update state UI terjadi pada UI thread.
- Worker berkomunikasi melalui message, bukan berbagi state mutable tanpa sinkronisasi.
- Gunakan bounded channel untuk event berfrekuensi tinggi.
- Shutdown menggunakan sinyal eksplisit dan `join` dengan strategi timeout.
- Operasi blocking harus memiliki batas atau dapat dibatalkan jika API mendukung.
- Setiap resource Windows dibungkus RAII agar handle tidak bocor saat error.
- Jangan memanggil `unwrap()` pada kondisi hardware/driver yang memang dapat gagal.

## 7. Error Handling dan Logging

- `thiserror` untuk error modul seperti `CollectorError`, `PdhError`, dan `ScanError`.
- `anyhow` hanya pada entry point atau boundary yang membutuhkan konteks.
- Log berisi konteks collector dan operasi, tetapi tidak membanjiri log setiap sampling.
- Warning berulang diberi throttling/backoff.
- Error pada satu adapter tidak menghapus data adapter lain.
- Jangan mencatat seluruh path file pada level log normal jika tidak diperlukan.

## 8. Pengujian

### Unit Test Dekat Modul

- Agregasi core aktif dan mapping logical processor.
- Parser dan agregasi instance GPU.
- Ring buffer histori.
- Formatter satuan dan validasi persentase.
- Kategori ekstensi file.
- Validasi konfigurasi.

### Integration Test

- Scanner pada folder sementara.
- Nested directory, Unicode path, empty file, large logical size, dan file tanpa ekstensi.
- Cancel scan sebelum selesai.
- Akses ditolak dan reparse point ketika dapat diuji dengan aman.
- Snapshot dari fixture PDH untuk single-GPU, hybrid-GPU, dan multi-process.

### Pengujian Manual Hardware

- Windows dengan bahasa non-Inggris.
- Intel/AMD/NVIDIA dan hybrid graphics.
- SATA/NVMe/USB, volume tanpa drive letter, dan network drive.
- Sleep/resume, perubahan interval, resize window, serta shutdown saat scan aktif.

Perintah dasar yang digunakan selama pengembangan:

```powershell
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

## 9. Konvensi Kode

- Gunakan nama modul sesuai domain, bukan nama vendor untuk model umum.
- Simpan ukuran internal dalam byte dan waktu dalam tipe `Duration`/`SystemTime` atau timestamp monotonic sesuai kebutuhan.
- Gunakan integer untuk kapasitas/byte dan floating point hanya untuk presentasi atau persentase.
- Hindari global mutable state.
- Fungsi murni untuk parsing dan agregasi agar mudah diuji.
- Unsafe code hanya berada pada FFI boundary yang memang membutuhkannya, disertai komentar safety.
- Tambah dependensi hanya jika memberikan manfaat nyata.
- Buat perubahan sekecil mungkin per tahap dan pertahankan kompatibilitas model UI saat collector berkembang.

## 10. Urutan Implementasi yang Disarankan

1. Inisialisasi crate, konfigurasi, app shell, dan navigasi.
2. Definisikan domain snapshot, channel, service lifecycle, dan mock collector.
3. Implementasikan history dan dashboard berbasis mock data.
4. Implementasikan CPU dan RAM Windows, lalu verifikasi terhadap Task Manager.
5. Implementasikan volume dan kapasitas storage.
6. Implementasikan PDH GPU, parser instance, memori, dan multi-GPU.
7. Tambahkan identitas physical disk dan aktivitas storage.
8. Implementasikan storage scanner beserta cancel/progress.
9. Tambahkan sensor vendor dan fitur P2 secara terpisah.
10. Optimasi berdasarkan profil nyata, lalu siapkan installer Windows.

Dengan urutan ini, UI dan lifecycle dapat diuji sejak awal tanpa menunggu seluruh integrasi hardware selesai.
