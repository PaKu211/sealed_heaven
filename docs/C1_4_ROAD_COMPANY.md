# C1.4 · Pendamping di jalan Qinghe–Willowford

Bagian Stage E ini menghubungkan rekrutmen, otonomi pendamping, bekal obat, ingatan sosial, dan kewajiban wakil Reed Hall. Sasaran GDD yang dikerjakan berasal dari §12.5 dan kriteria variasi sejarah sekte di dokumen rekonsiliasi.

## Cara mencoba

Mulai kampanye baru. Di sekitar ladang herba Qinghe, temui **Road outrider** (ID 22). Ketika bersebelahan, gunakan `talk 22` lalu `invite 22`. Petik herba di ladang dengan `gather`, lalu berikan satu dengan `pack 22`. Kantung pribadinya memuat dua herba. Berjalanlah menjauh dari rute patrolinya. Pendamping menggunakan gilirannya untuk menyusul, beristirahat, atau merawat pemain yang terluka. Satu perawatan menghabiskan satu herba dan memulihkan hingga dua HP.

Gunakan `part 22` saat bersebelahan untuk berpisah. Pendamping kembali menjalankan kegiatannya dan membawa bekalnya. `journal` menampilkan delapan kejadian perjalanan terakhir yang disaksikan pemain. `inspect 22` memperlihatkan isi kantung saat pendamping ada di sebelah pemain. Di tampilan piksel, **T** memilih target terdekat secara bergiliran, **E** mengajak bicara, **N** mengundang, **O** memberi bekal, dan **R** mengakhiri perjalanan.

Pilihan lain bagi anggota Reed Hall adalah **Luo Xiu** (ID 34). Bila guru gugur, jabatan menjadi kosong. Luo dapat mengambil jabatan setelah melihat jenazah atau menerima kabar mengenai insiden fatal itu. Dengan demikian, waktu dan tempat kematian memengaruhi berapa lama balai kehilangan guru dan apakah Luo masih menemani pemain. Menemukan jenazah saja tidak memberi Luo pengetahuan tentang pelaku. Wakil yang naik jabatan masih harus kembali secara fisik untuk mengajar.

## Keputusan arsitektur

Perluas `living::World::step` dan perencana tujuan NPC. Setiap calon pendamping memiliki aturan undangan di data skenario. Satu perjanjian perjalanan menunjuk ke ID NPC yang sudah ada. Urutan giliran tetap dimulai dengan kedatangan kabar, lalu tindakan pemain. Dunia memperbarui jabatan dan persepsi pendamping, memilih tujuan dari satu keadaan dunia, menjalankan aksi menurut ID, memeriksa kematian pemain yang terlihat, dan mengantrekan kabar baru.

Saat kontak visual putus, tujuan pencarian berasal dari posisi terakhir yang benar-benar dilihat. NPC mengakhiri pencarian setelah 12 giliran tanpa kontak. Hubungan yang berubah menjadi bermusuhan memutus perjanjian. Kebutuhan berobat, kelelahan, dan laporan penting dapat menyela kegiatan mengikuti pemain. Jurnal dunia mencatat penyebab, lokasi, giliran, dan apakah pemain menyaksikannya. Riwayat jabatan menyimpan kekosongan dan pengangkatan secara terpisah.

Checkpoint revisi 5 merekam input dan memverifikasi hasil replay terhadap digest yang mencakup perjanjian, persepsi terakhir, kantung tiap aktor, obat terpakai, serta riwayat perjalanan dan jabatan. Pembaca revisi 1–4 memilih aturan semula. Kampanye lama dapat dilanjutkan dengan aturan versi asal. Kampanye baru menggunakan revisi 5.

## Review dan batas verifikasi

Review menemukan bahwa stok herba yang sedang dibawa ke pasar bisa menahan pekerjaan pemasok kuil. Pada revisi 5, perencana dan pelaksana pekerjaan sama-sama memakai kekurangan kuil setelah memperhitungkan muatan pemasok dan kurir menuju Qinghe. Bekal pribadi disimpan terpisah dari muatan ekonomi.

Tes Rust mencakup dua rute dari input pemain, pemuatan di tengah perjalanan, pengulangan giliran, kapasitas kantung, pengobatan dan kematian dalam satu giliran, kehilangan jejak, perbedaan saksi, pengangkatan melalui laporan tertunda, reservasi stok, privasi terminal, dan penolakan input baru di revisi 4. Vektor pembuka revisi 4 disusun mandiri dari tata letak lama. Metode yang sama mereproduksi vektor revisi 3 yang sudah tersimpan.

Kompilasi, eksekusi tes Rust, playtest ASCII/piksel, dan pengukuran durasi pemuatan masih tertunda. Belum diketahui apakah batas pencarian 12 giliran dan siklus istirahat terasa tepat bagi pemain. Pemulihan masih memutar ulang dari awal dengan batas 65.536 giliran. Stage E masih membuka persaingan antarsekte, agenda faksi regional, dan warisan lintas generasi.
