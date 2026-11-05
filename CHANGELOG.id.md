# Catatan perubahan

## Belum dirilis

- Buka beberapa proyek sekaligus, masing-masing di tab sendiri di bilah judul. Tiap proyek punya file, panel Git dan terminalnya sendiri, jadi agent tetap jalan di satu proyek saat kamu melihat proyek lain.

## 0.1.0

Rilis pertama KlikVibe, editor kode ringan untuk vibe coding.

- Buka folder proyek (atau jalankan `klikvibe <folder>`) dan buka lagi dari daftar proyek terakhir.
- Pohon file yang mengikuti `.gitignore` dan isi disk: file yang baru berubah berkedip, dan file yang berubah menampilkan status Git-nya.
- Tab kode dengan CodeMirror 6 dan pewarnaan sintaks untuk bahasa-bahasa umum; file yang terbuka dimuat ulang saat agen mengubahnya, dan bertanya dulu bila ada editan Anda yang belum disimpan.
- Panel Git: perubahan dengan diff, commit semua, pull (hanya fast-forward), push atau publikasikan branch, riwayat dengan tanda commit yang belum di-push, dan `git init`.
- Terminal di folder proyek, sebanyak yang Anda perlukan, dengan tombol yang mengetikkan `claude`, `codex`, atau `gemini`.
- Pembaruan: KlikVibe memeriksa sekali sehari dan memasang versi baru setelah Anda setuju.
- Bahasa Inggris dan Bahasa Indonesia.
