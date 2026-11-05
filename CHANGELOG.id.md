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
- Graph commit di riwayat Git: garis branch dan merge, label branch dan tag, penanda commit yang sedang dipakai, dan pilihan antara branch ini atau semua branch.

## 0.1.0

Rilis pertama KlikVibe, editor kode ringan untuk vibe coding.

- Buka folder proyek (atau jalankan `klikvibe <folder>`) dan buka lagi dari daftar proyek terakhir.
- Pohon file yang mengikuti `.gitignore` dan isi disk: file yang baru berubah berkedip, dan file yang berubah menampilkan status Git-nya.
- Tab kode dengan CodeMirror 6 dan pewarnaan sintaks untuk bahasa-bahasa umum; file yang terbuka dimuat ulang saat agen mengubahnya, dan bertanya dulu bila ada editan Anda yang belum disimpan.
- Panel Git: perubahan dengan diff, commit semua, pull (hanya fast-forward), push atau publikasikan branch, riwayat dengan tanda commit yang belum di-push, dan `git init`.
- Terminal di folder proyek, sebanyak yang Anda perlukan, dengan tombol yang mengetikkan `claude`, `codex`, atau `gemini`.
- Pembaruan: KlikVibe memeriksa sekali sehari dan memasang versi baru setelah Anda setuju.
- Bahasa Inggris dan Bahasa Indonesia.
