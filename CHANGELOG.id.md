# Catatan perubahan

## Belum dirilis

- Buka beberapa proyek sekaligus, masing-masing di tab sendiri di bilah judul. Tiap proyek punya file, panel Git dan terminalnya sendiri, jadi agent tetap jalan di satu proyek saat kamu melihat proyek lain.
- Gambar, video, audio dan PDF tampil sebagai pratinjau, bukan "file biner", dan ikut diperbarui saat filenya berubah. SVG bisa dialihkan ke kodenya, dan semua pratinjau bisa dibuka di aplikasi bawaan.
- Klik kanan tab file untuk menutupnya, menutup tab lain, tab di kanannya, tab yang sudah disimpan atau semuanya, menyalin path, atau menampilkannya di Finder atau Explorer. Tab proyek punya menu yang sama untuk proyek.
- Klik kanan perubahan di panel Git untuk membukanya, membuang perubahannya (atau menghapus file baru), menambahkannya atau foldernya ke .gitignore, menyalin path, atau menampilkannya di Finder. Klik kanan commit untuk menyalin hash atau pesannya.
- Klik kanan file dan folder di pohon file, serta proyek di daftar terakhir, untuk menu serupa.
- Server MCP lokal (nonaktif secara bawaan) untuk agen yang kamu pakai: agen bisa membaca file dan teks yang sedang kamu pilih serta editan yang belum disimpan, dan membuka file dan diff di KlikVibe. Ada langkah setup untuk Claude Code, Codex, Gemini CLI, OpenCode, Oh My Pi, Command Code, Amp, Qwen Code, Kimi CLI, Droid, Copilot CLI, Crush, dan Cursor CLI.
- *Jalankan agen* mengenal lebih banyak agen: OpenCode, Oh My Pi (`omp`), Command Code (`cmd`), Amp, Qwen Code, Kimi CLI, Droid, Copilot CLI, Crush, Cursor CLI, dan Aider.
- Terminal yang disembunyikan meninggalkan strip di bawah editor untuk membukanya lagi, dan bilah judul menampilkan ikon terminal.
- Pembaruan seperti di KlikPaper: pasang otomatis (diunduh di latar, dipasang saat aplikasi ditutup), tanya dulu, atau hanya periksa saat diminta. Versi baru menampilkan perubahannya dalam bahasamu, dan setelah update KlikVibe sekali menampilkan apa saja yang baru.
- Jendela *Tentang & pembaruan* (tombol ⓘ di bilah judul, atau versi di Home) berisi versi, pilihan pembaruan, dan tombol Periksa pembaruan.
- Tata letak berdampingan: Terminal | Kode | Git sebagai kolom (tombol di sebelah tombol panel). Semua panel bisa disembunyikan, termasuk kode (`⌘⇧E` / `Ctrl+Shift+E`), sehingga terminal bisa memenuhi jendela; minimal satu tetap tampil. Pohon file tersembunyi secara bawaan di tata letak ini (`⌘B` untuk menampilkannya), dan tiap tata letak mengingat pilihannya sendiri. Terminal tetap berjalan saat tata letak diganti.
- Kartu sambutan tetap di belakang terminal saat terminal ditarik ke atas.
- Tooltip di semua tombol: arahkan kursor ke ikon untuk melihat fungsinya dan pintasannya.
- Graph commit di riwayat Git: garis branch dan merge, label branch dan tag, penanda commit yang sedang dipakai, dan pilihan antara branch ini atau semua branch. Klik commit untuk melihat daftar file yang diubah (dengan jumlah baris ditambah dan dihapus) dan membuka diff tiap file; commit lengkapnya tinggal satu klik lagi.
- Diff berdampingan: versi lama di kiri, versi baru di kanan, dua versi baris yang diedit sejajar dengan bagian yang berubah disorot. Tombol di header diff mengembalikan ke satu kolom.
- Pertanyaan seperti "Buang perubahan?" memakai dialog KlikVibe sendiri, bukan dialog sistem; untuk tindakan yang tidak bisa dibatalkan tombolnya merah dan fokus ada di Batal.
- Tekstur butiran film tipis di atas jendela dihapus: hampir tidak terlihat dan memakan sekitar 40 MB memori grafis saat KlikVibe di depan.
- Klik kanan di editor kode untuk Potong, Salin, Tempel, Pilih semua, Cari dan Ke baris, plus Simpan, perubahan file, path dan Tampilkan di Finder; di diff untuk Salin, Pilih semua, ganti tampilan dan file-nya.
- Pencarian di panel Files: berdasarkan nama (`⌘P`), dengan kata, pola `*`, `**`, `?`, dan `!` untuk mengecualikan, atau di isi file (`⌘⇧F`), bisa dibatasi ke file seperti `*.ts`; hasil membuka file di baris itu. Keduanya mengikuti `.gitignore`.
- Pintasan tidak lagi terpicu oleh Ctrl di terminal: di macOS hanya memakai ⌘, di sistem lain Ctrl+huruf diteruskan ke shell selama terminal menerima ketikan (Ctrl+W tidak lagi menutup tab file).
- Ikon file mengikuti jenis filenya: kode dengan warna bahasanya, JSON, konfigurasi, lockfile, `.env`, gambar, video, audio, arsip, `package.json`, `Cargo.toml`, `Dockerfile`, README dan LICENSE punya ikon sendiri, di pohon file, hasil pencarian dan tab.
- KlikVibe memberi tahu saat agen butuh kamu: setelah bekerja lalu diam (menunggu jawaban, atau selesai), atau membunyikan bel terminal atau mengirim notifikasi. Kalau kamu sedang tidak melihat terminal itu, tab proyeknya diberi ikon lonceng, ikon di Dock memantul dan macOS menampilkan notifikasi. Tombol lonceng di header terminal untuk mematikannya.
- Path di output terminal (`src/app.ts:42:7`, `--> src/main.rs:10:5`, `app.ts(12,5)`) bisa diklik untuk membuka file di baris itu, kalau filenya ada di proyek; alamat web seperti `http://localhost:5173` terbuka di browser.
- Tab terminal diberi nama sesuai program yang berjalan di dalamnya ("Claude Code", "npm run dev", "vite"), dan kembali ke "Terminal 1" saat di prompt shell. Klik dua kali atau klik kanan tab untuk mengganti nama; nama buatanmu tetap dipakai sampai kamu memilih nama otomatis lagi.
- Breadcrumb di atas editor (proyek › folder › file): folder menampilkan isinya untuk membuka file, file menawarkan Tampilkan di Files, path-nya dan Tampilkan di Finder. Tampilkan di Files membuka folder-foldernya di pohon file.
- Item menu klik kanan kembali tampil dengan huruf biasa dan garis pemisahnya utuh; panah di breadcrumb tampil dengan benar.
- Checkpoint: selama agen bekerja, KlikVibe menyimpan snapshot file setiap kali file berhenti berubah sejenak (dan paling lambat tiap menit), di ref Git tersendiri yang tidak pernah menyentuh commit, branch, index, atau stash-mu, dan tidak pernah ter-push. Tab Checkpoint di panel Git menampilkannya beserta file dan diff-nya; Kembalikan mengembalikan file seperti saat itu (kondisi saat ini disimpan dulu, jadi pengembalian pun bisa dibatalkan).
- Tanya agen: klik kanan kode (baris yang dipilih), file, folder, diff, perubahan, atau commit lalu pilih "Tanya agen tentang ini"; rujukan seperti `@src/app.ts baris 12-30` ditempel ke terminal agen untuk kamu lengkapi dan kirim. Tombol Prompt di terminal berisi prompt siap pakai dan prompt buatanmu.
- MCP: enam tool baru untuk agen. `read_terminal` (output terbaru terminal lain, misalnya error dev server), `create_checkpoint` (dengan catatan), `list_checkpoints`, `restore_checkpoint` (hanya setelah kamu setujui di KlikVibe), `show_commit`, dan `reveal_in_files`. Checkpoint bisa diberi catatan, yang tampil di tab Checkpoint.
- KlikVibe terbuka lagi seperti saat ditinggalkan: proyek, tab file, dan proyek yang di depan, termasuk setelah update.
- Buka di GitHub, GitLab, atau Bitbucket: repositori, branch, commit, atau file di baris yang dipilih (dari panel Git, History, editor, dan breadcrumb); setelah push branch fitur, satu klik membuat pull (merge) request. Bisa untuk remote SSH dan HTTPS serta GitLab self-hosted.

## 0.1.0

Rilis pertama KlikVibe, editor kode ringan untuk vibe coding.

- Buka folder proyek (atau jalankan `klikvibe <folder>`) dan buka lagi dari daftar proyek terakhir.
- Pohon file yang mengikuti `.gitignore` dan isi disk: file yang baru berubah berkedip, dan file yang berubah menampilkan status Git-nya.
- Tab kode dengan CodeMirror 6 dan pewarnaan sintaks untuk bahasa-bahasa umum; file yang terbuka dimuat ulang saat agen mengubahnya, dan bertanya dulu bila ada editan Anda yang belum disimpan.
- Panel Git: perubahan dengan diff, commit semua, pull (hanya fast-forward), push atau publikasikan branch, riwayat dengan tanda commit yang belum di-push, dan `git init`.
- Terminal di folder proyek, sebanyak yang Anda perlukan, dengan tombol yang mengetikkan `claude`, `codex`, atau `gemini`.
- Pembaruan: KlikVibe memeriksa sekali sehari dan memasang versi baru setelah Anda setuju.
- Bahasa Inggris dan Bahasa Indonesia.
