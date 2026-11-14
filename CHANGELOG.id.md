# Catatan perubahan

## 0.7.19

Lihat bagian proyek mana yang berubah: file dikelompokkan per folder di panel Git.

- Changes, History dan Checkpoints mengelompokkan file di bawah folder teratasnya (backend, frontend, ...). Klik folder untuk melipatnya; di Changes, stage atau unstage satu folder sekaligus dari barisnya.

## 0.7.18

Terminal pas di panelnya sejak pertama dibuka.

- Terminal tidak lagi melewati batas bawah panel saat KlikVibe pertama dibuka, atau setelah jendela dipindah ke layar lain.

## 0.7.17

Daftar lebih rapi: bersihkan proyek terakhir, buka atau tutup isi satu folder sekaligus.

- Beranda punya tombol Bersihkan untuk daftar proyek terakhir. Proyek yang sedang terbuka tetap ada di daftar.
- Klik kanan folder di Files untuk membuka atau menutup semua isinya, tanpa mengubah folder lain.

## 0.7.16

Lihat sekilas di mana agenmu berjalan.

- Tab proyek menampilkan berapa terminal yang sedang menjalankan agen (Claude Code, Codex, ...). Shell biasa atau dev server tidak dihitung, dan tandanya hilang saat agennya ditutup.

## 0.7.15

Seret file dari Finder atau Explorer, seperti di editor lain.

- Seret file dari Finder atau Explorer ke jendela untuk membukanya di editor, siap diedit dan disimpan, termasuk file di luar proyek. Seret folder untuk membukanya sebagai proyek. Kalau dijatuhkan di terminal, path-nya diketik di sana.
- File dari luar proyek menampilkan path lengkapnya di atas editor (mulai dari ~, folder home), dengan tanda "di luar proyek". Klik salah satu bagiannya untuk menampilkannya di Finder atau menyalin path-nya.

## 0.7.14

Perbaikan untuk palet perintah.

- Palet perintah: nama perintah tampil dengan huruf biasa lagi. Di 0.7.13 nama perintah tampil dalam huruf kapital kecil sehingga sulit dibaca.

## 0.7.13

Palet perintah, tema terang, dua terminal berdampingan, dan stage satu bagian file. Lewat KlikVibe Remote, kini kamu bisa commit dan push dari HP.

- Palet perintah: ⌘⇧P (Ctrl+Shift+P di Windows dan Linux) menemukan apa saja yang bisa dilakukan KlikVibe, dari menampilkan panel sampai Push atau mengirim prompt ke agen. Ketik beberapa huruf lalu tekan Enter; perintah yang terakhir dipakai muncul paling atas.
- Tema terang: pilih Gelap, Terang, atau Ikuti komputer di halaman Daftar proyek (di atas pilihan bahasa) atau lewat palet perintah. Editor, diff, dan terminal ikut berubah.
- Ukuran teks: ⌘+ dan ⌘− (Ctrl di Windows dan Linux) memperbesar atau memperkecil teks editor dan terminal, ⌘0 kembali ke ukuran bawaan.
- Terminal:
  - Dua terminal berdampingan, misalnya agen di samping dev server: tombol baru di kepala terminal, atau "Tampilkan di samping terminal yang terbuka" di menu klik kanan tab. Seret garis di antaranya untuk memberi salah satunya ruang lebih; klik dua kali untuk kembali setengah-setengah.
  - Cari di output dengan ⌘F (Ctrl+Shift+F di Windows dan Linux) atau tombol cari: hasilnya disorot, Enter dan Shift+Enter pindah ke hasil berikutnya dan sebelumnya.
  - Tab yang agennya sedang menunggumu menampilkan ikon lonceng sampai tab itu dibuka. Mengklik notifikasi agen membuka terminalnya.
  - Seret file atau folder dari pohon file ke terminal untuk mengetikkan path-nya di sana (dari folder project, diberi tanda kutip bila perlu).
  - Tab bisa diseret untuk mengubah urutannya, dan menampilkan tombol × saat di-hover dan di tab yang terbuka. Terminal terakhir juga bisa ditutup: terminal baru langsung menggantikannya.
  - Tombol "Jalankan agen" dan daftar agennya dihapus: jalankan agen dengan mengetik perintahnya.
- Prompt: setiap prompt di menu Prompt bisa diubah atau dihapus, termasuk prompt bawaan ("Atur prompt…"). "Kembalikan prompt bawaan" memunculkan lagi yang sudah dihapus.
- Git: diff sebuah file punya tombol Stage (atau Unstage) dan Buang di tiap bagian perubahan, untuk meng-commit atau membuang bagian itu saja, dengan Urungkan setelah membuang. Bagian yang sudah di-stage diberi tanda.
- Tab: tombol ‹ dan › menggeser tab project, terminal, dan file yang tidak muat, dan tab yang terbuka otomatis tergeser agar terlihat.
- Files: tombol di sebelah Muat ulang menutup semua folder, atau membuka semuanya kalau semua tertutup (kecuali yang diabaikan .gitignore, seperti node_modules). Klik dua kali tepi pohon file, panel Git, atau terminal mengembalikannya ke ukuran bawaan.
- Halaman Daftar proyek tidak lagi menampilkan daftar agen yang didukung KlikVibe: semua agen yang berjalan di terminal bisa dipakai.
- KlikVibe Remote (dengan KlikVibe Remote 1.0.3 di HP):
  - Layar Perubahan bisa stage file, commit, dan push, dan diff sebuah file bisa stage atau membuang satu bagian perubahan (dengan Urungkan).
  - HP menerima menu Prompt terminal sesuai yang kamu atur di sini, dan menampilkannya di samping tombol-tombol keyboard.

## 0.7.12

Serangkaian perbaikan: panel Git di project yang berupa subfolder, daftar Git yang panjang, ganti nama dan link di pohon file, dan KlikVibe Remote yang lebih stabil.

- Panel Git:
  - Project yang berupa subfolder dari sebuah repositori hanya menampilkan dan mengubah file miliknya sendiri, dengan path yang benar. Sebelumnya path dihitung dari puncak repositori, jadi membuka, men-stage, atau membuang perubahan sebuah file bisa gagal, dan "Commit semua" ikut mengambil file di luar project.
  - Daftar yang panjang (banyak file berubah, riwayat panjang) dibaca utuh. Sebelumnya daftar yang terpotong di tengah bisa menampilkan entri terakhir yang rusak.
  - Diff yang terbuka mengikuti file-nya: dimuat ulang setelah commit atau buang perubahan, dan tab-nya tetap ada saat file diganti nama. File tanpa perubahan menampilkan diff kosong, bukan error.
  - Commit merge menampilkan diff-nya (diff gabungan sebelumnya tampil sebagai teks biasa).
  - File dengan karakter tak biasa di namanya (`*`, `[`, `:`) di-diff sebagai file itu, bukan sebagai pola.
- Checkpoint: folder yang merupakan repositori Git tersendiri (submodule, atau clone di dalam project) tidak lagi membuat checkpoint gagal. Folder itu dilewati, tab Checkpoint mencantumkannya, dan saat menghapusnya dikatakan bahwa itu tidak bisa dibatalkan.
- Pohon file: mengganti nama file hanya beda huruf besar-kecil (`readme.md` jadi `README.md`) berfungsi di Mac dan Windows. Mengganti nama atau menghapus link berlaku pada link-nya, bukan pada yang ditunjuknya. Pencarian yang sudah dikosongkan tidak lagi menampilkan hasil yang terlambat.
- Terminal: menempel teks besar ke terminal yang sedang sibuk tidak lagi membekukan jendela.
- Pembaruan: dengan pembaruan otomatis dimatikan, pembaruan yang sudah diunduh tidak dipasang saat keluar. Suntingan yang diketik selama pembaruan diunduh ditanyakan dulu sebelum project ditutup.
- Preview: tab mengikuti alamat yang dikirim lagi kepadanya.
- KlikVibe Remote:
  - Mengganti kunci pemasangan saat HP sedang terhubung tidak bisa lagi membuat komputer dan HP memegang kunci yang berbeda.
  - Pesan terakhir ke HP (misalnya "akses dicabut") sampai sebelum koneksi ditutup.
  - HP yang sedang menunggu izin saat Remote dimatikan, atau saat HP lain meminta, ditolak dan tidak dibiarkan menunggu.
  - Pengaturan yang diubah pada saat bersamaan tidak lagi saling menimpa.
  - Tombol Remote dan MCP menampilkan keadaan sebenarnya setelah sebuah perubahan gagal.

## 0.7.11

Pembaruan yang tidak bisa menghilangkan project-mu, dan KlikVibe Remote yang lebih stabil.

- Memasang pembaruan kini mengunduhnya dulu, dengan project-mu masih terbuka, dan menanyakan suntingan yang belum disimpan di setiap project sebelum ada yang ditutup. Sebelumnya, unduhan yang gagal (internet lambat atau mati) meninggalkan KlikVibe tanpa project terbuka dan tanpa yang bisa dipulihkan, mempertahankan suntingan di satu project tetap menutup yang lain, dan percobaan kedua selalu gagal dengan "No update to install".
- Di pencarian file, ↑ dan ↓ menggulir hasil project tempatmu berada (dengan pencarian project lain terbuka, yang tergulir malah yang itu).
- KlikVibe Remote:
  - Setelah Mac bangun dari tidur, relay tersambung lagi dalam hitungan detik (sebelumnya bisa 90 detik sambil tetap tampil terhubung).
  - HP yang meninggalkan Wi-Fi dilepas setelah satu menit, tidak lagi tampil terhubung.
  - Daftar Git di HP muat untuk working tree yang sangat besar (path panjang tidak lagi mengubahnya jadi error).
  - "Putuskan dari akun" saat offline memberi tahu dan mempertahankan tautannya, tidak lagi meninggalkan komputer di akun tanpa cara menghapusnya.
  - HP menampilkan project mana yang membutuhkanmu walau notifikasi desktop dimatikan, dan mempertahankan terminal yang shell-nya sudah berakhir (untuk membaca keluaran terakhirnya, atau menutupnya).
  - Di koneksi lambat, perubahan pada daftar terminal tidak lagi terlewat.
- Pohon file bisa menerima fokus keyboard (aksesibilitas).

## 0.7.10

Suntingan yang belum disimpan kini aman, dan perbaikan untuk file Windows, worktree, dan Publikasikan.

- Keluar (⌘Q) atau menutup jendela dengan suntingan yang belum disimpan kini bertanya dulu. Sebelumnya KlikVibe langsung tertutup dan suntingannya hilang.
- File dengan akhir baris Windows (CRLF) mempertahankannya saat disimpan. Sebelumnya, menyimpan mengubah semua akhir baris jadi LF (sehingga Git menampilkan seluruh file sebagai berubah), dan suntingan agen pada file seperti itu menandai tab-nya belum disimpan.
- "File ini berubah di disk" hanya muncul kalau teks file benar-benar berubah: tidak lagi setelah kamu sendiri menyimpan sambil terus mengetik, atau setelah `npm install` atau build menyentuh banyak file. Ketikan saat file sedang disimpan tidak lagi dianggap sudah tersimpan.
- Menghapus file yang dikecualikan .gitignore (seperti `.env`) mengatakan bahwa itu tidak bisa dibatalkan: checkpoint tidak menyimpan file tersebut. Menghapus folder mengatakan bahwa file gitignore di dalamnya tidak akan kembali.
- Di worktree sebuah tugas (agen paralel), panel Git mengikuti commit yang dibuat di sana, jadi Merge tidak lagi meminta commit dulu.
- Publikasikan melakukan push ke remote repositori walau namanya bukan `origin`.
- Tool MCP menerima path Windows dengan garis miring mana pun dan huruf besar-kecil apa pun.
- KlikVibe Remote: terminal yang ukurannya diubah di komputer tampil dengan ukuran barunya di HP (sebelumnya layarnya berantakan sampai terminal dibuka lagi).

## 0.7.9

Tidak ada lagi "Dev server berjalan" untuk alamat milik agen sendiri.

- Diperbaiki: terminal yang menjalankan agen (Claude Code, Codex, ...) menawarkan preview untuk alamat lokal apa pun yang dicetak agen, seperti server milik sebuah plugin (http://localhost:37777 milik claude-mem), setiap kali project dibuka. Kini hanya alamat yang dicetak di luar agen yang menawarkannya, seperti dev server yang dijalankan dari terminal atau menu Script.

## 0.7.8

Salin dan tempel di terminal, tanpa bunyi bip.

- Diperbaiki: di macOS, ⌘C di terminal menyalin tetapi membunyikan suara "tidak bisa" dari sistem. Kini menyalin tanpa suara.
- Windows dan Linux, seperti di Windows Terminal: Ctrl+C menyalin saat ada teks terpilih di terminal (dan menginterupsi seperti biasa saat tidak ada), Ctrl+Shift+C selalu menyalin, dan Ctrl+Shift+V menempel (Ctrl+V juga di Windows). Sebelumnya tombol-tombol ini hanya diteruskan ke shell.

## 0.7.7

KlikVibe Remote: langsung kembali ke relay.

- Diperbaiki: setelah sebuah komputer dihapus dari akun lalu ditambahkan lagi, HP yang jauh dari Wi-Fi-nya tidak bisa menjangkaunya sampai 5 menit (tampil online, tetapi "tidak terjangkau"). Relay kini tersambung lagi begitu komputer ditambahkan kembali.

## 0.7.6

KlikVibe Remote: pemasangan lebih aman, dan relay yang tidak menyerah.

- Mengizinkan sebuah HP menampilkan kode enam digit, dan HP menampilkan kodenya sendiri setelah scan. Izinkan hanya kalau keduanya sama: orang yang melihat kode QR-mu tidak bisa lagi menyamar sebagai HP-mu.
- Kunci di kode QR hanya berlaku sekali: saat HP terhubung, ia mendapat kunci baru dan kunci di kode QR berhenti berfungsi.
- HP yang ditambahkan lagi dengan kunci baru mengakhiri koneksi yang masih memakai kunci lama.
- Di Wi-Fi, Remote hanya menerima koneksi dari alamat lokal, beberapa saja sekaligus dari tiap alamat, dan menutup yang menganggur, jadi perangkat lain di jaringan tidak bisa membuatnya sibuk terus.
- Kalau koneksi relay gagal karena bug, panel Remote memberi tahu, error-nya dilaporkan, dan dicoba lagi semenit kemudian (sebelumnya berhenti sampai KlikVibe dijalankan ulang).

## 0.7.5

KlikVibe Remote: dari mana saja lagi.

- Diperbaiki: dengan Pro, HP yang jauh dari Wi-Fi komputer (misalnya memakai data seluler) tidak bisa menjangkaunya ("tidak terjangkau"). Sambungan komputer ke relay gagal pada percobaan pertama dan tidak pernah kembali; kini tersambung dan tetap tersambung.

## 0.7.4

KlikVibe Remote: notifikasi push menyebut apa yang terjadi.

- Notifikasi push (Pro) menyebut apa yang terjadi, dalam bahasa HP: "Menunggu jawaban, atau sudah selesai", "Butuh kamu" (notifikasi dari agen sendiri) atau "Meminta perhatianmu" (bel), beserta nama komputernya. Tetap tidak pernah berisi apa yang dikatakan agen.

## 0.7.3

KlikVibe Remote: seluruh riwayat di HP.

- HP yang membuka sebuah terminal mendapat riwayatnya seperti yang tampil di jendela (sampai 3000 baris, berwarna), bukan hanya keluaran mentah terakhir, yang cepat habis oleh gambar ulang agen. Kamu bisa menggulir ke atas sejauh di komputer.

## 0.7.2

KlikVibe Remote: tutup terminal dari HP.

- Tutup terminal dari HP (setelah ditanya): tab-nya tertutup di komputer, dan yang berjalan di dalamnya berhenti, seperti saat ditutup di sana.
- Klik kanan path di atas editor (project, folder, atau file-nya): Tampilkan di Finder (Tampilkan di Explorer), tampilkan di Files, salin path atau path relatifnya.

## 0.7.1

KlikVibe Remote: jalankan agen dan kirim gambar dari HP.

- Mulai terminal baru dari HP di project mana pun yang terbuka, dengan salah satu agenmu (Claude Code, Codex, Gemini, ...) atau shell biasa. HP hanya bisa memilih agen yang dikenal KlikVibe, tidak pernah mengirim perintahnya sendiri.
- Kirim gambar dari HP ke komputer dan serahkan path-nya ke agen. Gambar disimpan secara privat di folder sementara (`klikvibe-remote`) dan dihapus setelah seminggu.
- Notifikasi push (Pro) kini menyebut terminal mana yang membutuhkanmu, jadi mengetuknya membuka terminal itu, dan tombol Ya/Tidak menjawab agen. Push tetap tidak pernah membawa apa yang dikatakan agen.
- Butuh KlikVibe Remote 1.0.1 di HP; HP dengan versi lama tetap berfungsi seperti sebelumnya.

## 0.7.0

KlikVibe Remote: agen AI-mu, dari HP.

- Remote (ikon HP di kanan atas): pasangkan aplikasi KlikVibe di HP dengan scan kode QR, lalu pantau terminal dan agen semua project yang terbuka dari HP, balas, dan lihat perubahan serta diff-nya. Setiap HP diizinkan dulu di komputer, dan bisa dicabut di komputer maupun di aplikasi.
- Terenkripsi end-to-end (Noise NNpsk0, dengan kunci dari kode QR) di Wi-Fi yang sama, dan lewat relay KlikVibe dari mana saja dengan Pro; protokol terbukanya ada di `src-tauri/src/remote/PROTOCOL.md`.
- Agen yang menunggumu juga muncul di HP, walau notifikasi desktop dimatikan (dan sebagai notifikasi push dengan Pro).
- Untuk pengembangan: `npm run tauri:dev` menjalankan "KlikVibe Dev" terpisah di samping KlikVibe yang terpasang.

## 0.6.0

Ke definisi, versi ringan: ⌘-klik atau F12.

- Ke definisi, tanpa language server: ⌘-klik (Ctrl+klik) atau F12 pada sebuah nama langsung loncat ke tempat ia didefinisikan (function, class, struct, fn, def, const, method, ...), mendahulukan file asal import-nya; kalau ada beberapa kandidat, muncul daftar untuk dipilih. Di dalam tanda kutip import, file-nya langsung terbuka (./x, ../x, $lib/x, @/x, modul Python, `mod x;` Rust). Alt+klik kini menambah kursor.

## 0.5.3

⌘-klik di terminal makin luas: folder, dan nama file di bawah folder yang disebut di atasnya.

- ⌘-klik di terminal makin luas: folder absolut (/tmp/x/card/, ~/Projects/app) terbuka di Finder atau Explorer, dan nama file saja seperti `sheet-ios-feed.png` bisa dibuka kalau foldernya disebut beberapa baris di atasnya.

## 0.5.2

⌘-klik path di luar proyek di terminal.

- ⌘-klik (Ctrl+klik) kini juga membuka path di luar proyek yang ditulis agen, misalnya screenshot di folder sementara: gambar, video, audio, dan PDF terbuka di aplikasi bawaannya, selain itu ditampilkan di Finder atau Explorer. Path panjang yang terpotong ke baris berikutnya tetap terbaca utuh.

## 0.5.1

Terminal memakai seluruh tinggi selama tidak ada file terbuka.

- Tanpa file terbuka, terminal naik sampai atas, tidak lagi di bawah kartu sambutan; membuka file memunculkan kode lagi di atasnya, dengan tinggi terminal yang kamu atur, dan menutup tab terakhir memberi ruang itu kembali ke terminal.

## 0.5.0

Batas untuk checkpoint, dan berapa ruang yang dipakainya.

- Checkpoint punya batas: secara bawaan 100 terakhir disimpan, yang paling lama dihapus lebih dulu, dan tab Checkpoint menampilkan jumlah serta ukurannya (mis. "23 dari 100 · 3.8 MB"). Klik untuk menyimpan 50, 100, 200, 500, atau semuanya. Hapus checkpoint kini berikon tempat sampah.

## 0.4.1

Perbaikan KlikVibe yang tiba-tiba tertutup di macOS.

- Diperbaiki: KlikVibe bisa tiba-tiba tertutup di macOS saat dipakai. Error kecil di dalam sistem jendela macOS menghentikan aplikasi alih-alih dicatat; sekarang dicatat dan KlikVibe tetap berjalan.

## 0.4.0

Terminal kembali setelah restart dengan agen di percakapan terakhirnya, ⌘-klik file dan folder di terminal, dan tab yang bisa digeser.

- File dan folder yang disebut di terminal terbuka dengan ⌘-klik (Ctrl+klik di Windows dan Linux), termasuk folder, file yang dikecualikan .gitignore, dan nama saja seperti `SKILL.md` kalau hanya satu file proyek yang bernama begitu. File terbuka di editor (di barisnya), folder di pohon file. Alamat web terbuka dengan cara yang sama; klik biasa tidak lagi membukanya.
- Tab proyek dan tab file bisa digeser ke posisi lain di barisnya.
- Terminal kembali saat KlikVibe dibuka lagi: tab dan namanya sama, output terakhirnya (redup), dan agen yang tadi berjalan dijalankan lagi di percakapan terakhirnya (Claude Code, Codex, Gemini CLI, OpenCode, Kimi CLI, Copilot CLI, dan Cursor CLI; agen lain diketikkan untuk kamu jalankan). Menutup proyek melupakan terminalnya.

## 0.3.2

Title bar dalam mode layar penuh di macOS.

- Dalam mode layar penuh di macOS, title bar mulai dari tepi kiri, karena tombol jendela tersembunyi di sana.

## 0.3.1

Folder yang di-gitignore tampil di pohon file, ikon file di panel Git, dan tampilan installer baru.

- Pohon file kini menampilkan yang dikecualikan .gitignore (dist, node_modules, hasil build, aset store) dengan warna redup, bukan menyembunyikannya. Pencarian nama dan isi file tetap melewati file-file itu.
- Changes, file di commit, dan file di checkpoint pada panel Git memakai ikon tipe file yang sama dengan pohon file.
- Disk image macOS dan installer Windows (.exe dan .msi) memakai artwork KlikVibe, bukan tampilan bawaan yang polos.

## 0.3.0

Stage, unstage, dan batalkan tiap file di panel Git, serta buat, ganti nama, dan hapus file di pohon file.

- Staging di panel Git: setiap perubahan punya tombol untuk membatalkannya dan untuk men-stage-nya (atau unstage yang sudah di-stage), juga di menu klik kanannya. File yang di-stage punya bagian sendiri dengan Unstage semua, sisanya Stage semua, dan tombol commit hanya meng-commit yang di-stage ("Commit 2 staged"), atau semuanya kalau tidak ada yang di-stage.
- Files: klik kanan file atau folder (atau bagian kosong pohon, atau tombol di header-nya) untuk File baru, Folder baru, Ganti nama, dan Hapus. Nama baru boleh berisi folder (lib/util.ts); file baru langsung terbuka; tab yang terbuka ikut saat di-rename; checkpoint dibuat sebelum menghapus, jadi bisa dikembalikan.

## 0.2.0

Beberapa proyek sekaligus, checkpoint untuk membatalkan yang dilakukan agen, tugas paralel di worktree, server MCP untuk agenmu, dan banyak lagi untuk vibe coding.

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
- Checkpoint: selama agen bekerja, KlikVibe menyimpan snapshot file setiap kali file berhenti berubah sejenak (dan paling lambat tiap menit), di ref Git tersendiri yang tidak pernah menyentuh commit, branch, index, atau stash-mu, dan tidak pernah ter-push. Tab Checkpoint di panel Git menampilkannya beserta file dan diff-nya; Kembalikan mengembalikan file seperti saat itu (kondisi saat ini disimpan dulu, jadi pengembalian pun bisa dibatalkan).
- Tanya agen: klik kanan kode (baris yang dipilih), file, folder, diff, perubahan, atau commit lalu pilih "Tanya agen tentang ini"; rujukan seperti `@src/app.ts baris 12-30` ditempel ke terminal agen untuk kamu lengkapi dan kirim. Tombol Prompt di terminal berisi prompt siap pakai dan prompt buatanmu.
- MCP: enam tool baru untuk agen. `read_terminal` (output terbaru terminal lain, misalnya error dev server), `create_checkpoint` (dengan catatan), `list_checkpoints`, `restore_checkpoint` (hanya setelah kamu setujui di KlikVibe), `show_commit`, dan `reveal_in_files`. Checkpoint bisa diberi catatan, yang tampil di tab Checkpoint.
- KlikVibe terbuka lagi seperti saat ditinggalkan: proyek, tab file, dan proyek yang di depan, termasuk setelah update.
- Buka di GitHub, GitLab, atau Bitbucket: repositori, branch, commit, atau file di baris yang dipilih (dari panel Git, History, editor, dan breadcrumb); setelah push branch fitur, satu klik membuat pull (merge) request. Bisa untuk remote SSH dan HTTPS serta GitLab self-hosted.
- Script dan preview: tombol Script di terminal menjalankan script package.json (dengan bun, pnpm, yarn, atau npm sesuai lockfile) atau cargo run/test/build di terminal baru. Saat terminal menampilkan alamat dev server lokal, KlikVibe menawarkan untuk membukanya di tab preview (muat ulang, kolom alamat, buka di browser), dan tab terminalnya diberi ikon bola dunia untuk membukanya lagi.
- Yang belum kamu lihat: perubahan dari agen sejak terakhir kamu buka diff atau file-nya diberi titik di panel Git (dengan jumlah dan "Tandai semua sudah dilihat"), tombol Git di bilah judul diberi titik, dan checkpoint yang dibuat sejak terakhir kamu buka tab Checkpoint ditandai Baru. Simpanan dari kamu sendiri tidak dihitung.
- Tugas paralel: tombol branch di panel Git membuat tugas dengan branch dan folder sendiri (git worktree di sebelah repositori) lalu membukanya sebagai proyek dengan terminal sendiri, jadi satu agen bisa mengerjakannya sementara agen lain bekerja di folder utama. Dari tugas itu, Merge ke main membawa commit-nya kembali dengan merge commit, dan Hapus menghapus folder-nya (branch-nya tetap). Tiap folder punya checkpoint sendiri.

## 0.1.0

Rilis pertama KlikVibe, editor kode ringan untuk vibe coding.

- Buka folder proyek (atau jalankan `klikvibe <folder>`) dan buka lagi dari daftar proyek terakhir.
- Pohon file yang mengikuti `.gitignore` dan isi disk: file yang baru berubah berkedip, dan file yang berubah menampilkan status Git-nya.
- Tab kode dengan CodeMirror 6 dan pewarnaan sintaks untuk bahasa-bahasa umum; file yang terbuka dimuat ulang saat agen mengubahnya, dan bertanya dulu bila ada editan Anda yang belum disimpan.
- Panel Git: perubahan dengan diff, commit semua, pull (hanya fast-forward), push atau publikasikan branch, riwayat dengan tanda commit yang belum di-push, dan `git init`.
- Terminal di folder proyek, sebanyak yang Anda perlukan, dengan tombol yang mengetikkan `claude`, `codex`, atau `gemini`.
- Pembaruan: KlikVibe memeriksa sekali sehari dan memasang versi baru setelah Anda setuju.
- Bahasa Inggris dan Bahasa Indonesia.
