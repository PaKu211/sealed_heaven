# Sealed Heaven C1.6 — Perguruan, kesetiaan, dan ekonomi politik

**Status:** dikompilasi pada 20 November 2026. `living::` 50/50, ASCII 11/11, `sealed_agency` 5/5, Rustfmt dan Clippy `-D warnings` bersih. Belum ada playtest manusia lima menit, audit pembaca kedua, atau perbandingan byte revisi 6 terhadap ZIP C1.5.

Berbasis arsip C1.5 yang telah dirapikan dan, menurut laporan pemilik proyek, meluluskan 42 tes `living::`, 5 tes binary `sealed_agency`, dan 9 tes ASCII. Koreksi fixture penjaga yang memblokir celah dari arsip tersebut dipertahankan. C1.6 ialah bagian Stage E berikutnya, bukan klaim bahwa politik atau simulasi seluruh benua telah selesai.

## Apa yang berubah bagi pemain?

Persaingan Stonebank dan Reed Hall kini **dua arah**. Utusan Stonebank (ID 35, `V`) tetap bergerak, membeli herba dari pasar sesungguhnya, dan mengajukan tuntutan ke balai. **Qiao Lin, pengurus Reed Hall** (ID 36, `Q`), memiliki kas tersendiri sebanyak 18 koin. Ketika tekanan rival positif, ia bisa berjalan ke pasar Willowford, membeli herba pada harga yang juga dibayar pemain, lalu kembali ke balai dan menghabiskan herba itu sebagai pasokan. Setiap pasokan yang benar-benar sampai menaikkan ledger persembahan balai dan mengurangi tekanan Stonebank sebesar satu.

NPC pembeli tidak dapat menciptakan herba atau koin, tidak dapat membeli saat pasar tutup, dan tetap memakai prioritas kelelahan, ancaman, laporan, pengobatan, rekrutmen, dan gerakan satu petak per giliran. Dua pihak dapat saling membatasi stok dan membelanjakan kas yang terbatas. Tidak ada pemindahan kargo jarak jauh hanya karena terjadi perubahan angka pengaruh.

Pemain mendapat jalan baru selain `accord` dan `sponsor`:

| Perintah | Syarat | Dampak |
| --- | --- | --- |
| `talk 35`, lalu `pledge` | Bersebelahan dengan utusan, tidak bermusuhan, satu herba; titipan balai harus sudah diselesaikan | Bersumpah pada Stonebank, memberikan herba fisik kepada utusan, dan melepas keanggotaan Reed Hall |
| `aidlodge` | Anggota Stonebank, bersebelahan dengan utusan, satu herba | Memasok utusan, memperkuat tuntutan dan kepercayaan Stonebank |
| `renounce` | Anggota Stonebank, di dekat Reed Hall | Membatalkan sumpah dengan kehilangan dukungan Stonebank; memungkinkan jalan kembali ke Hall |
| `sponsor` | Anggota Reed Hall, dekat balai, satu herba | Mengurangi tekanan rival dan meningkatkan kepercayaan Reed Hall |
| `invite 35` | Anggota Stonebank, sudah berkenalan dan membantu perguruan | Utusan dapat menjadi pendamping; pekerjaannya di pasar dan balai tersela selama mendampingi |
| `invite 36` | Anggota Reed Hall, sudah berkenalan, pernah mendukung balai | Pengurus balai dapat menjadi pendamping; distribusi bantuan Hall tersela |

Satu pendamping tetap menjadi batas sistem, dan perekrutan mengikuti hubungan sosial yang telah ada. Meninggalkan Stonebank memutus perjanjian perjalanan dengan utusannya. Keanggotaan Stonebank juga menutup pelayanan guru Reed Hall sampai pemain melepaskan sumpah. Jurus yang telah dipelajari tidak dihapus saat berpindah kesetiaan, tetapi tidak berarti pengajaran Reed Hall tetap tersedia.

Perintah `factions` menampilkan kondisi lokal dan dukungan yang didapat pemain hanya setelah mendatangi Willowford; kas rahasia dua pihak ditampilkan hanya di developer view. `factionlog` mempertahankan riwayat C1.5; `politicslog` menampilkan pembelian, pengiriman, sumpah, bantuan, dan pembatalan sumpah **yang dilihat sendiri**. Komando inspeksi dan kedua jurnal tidak memajukan tick.

## Kompatibilitas dan kontrak simulasi

Kampanye bawaan kini `World::sect_politics(seed)` dengan **checkpoint revisi 7**. Mesin tetap murni, integer, satu tick per perintah, actor ID stabil, `BTreeMap` terurut, tanpa model AI saat runtime. Kernel C0.1 dan formasi lama C0.8–C1.5 tidak diubah.

- Header tetap `SH:C10:WORLD`. Tag intent baru 23–25 hanya diterima pada revisi 7, sedangkan tag 21–22 diizinkan pada revisi 6 dan 7. Kode goal, role, invitation, outcome, dan rejection baru ditambahkan tanpa mengubah kode lama.
- Domain rules/digest memakai penanda `SH:C10:RULES:v7` dan `SH:C10:STATE:v7`. Digest baru menghitung seluruh kas, pembelian, pengiriman, reputasi, sumpah, event politik, tanda saksi, serta status dan kargo NPC. Checkpoint masih **replay dari jurnal**, bukan snapshot mutable yang menyimpang dari jalur simulasi kanonis.
- Revisi 1–6 tetap memilih ruleset dan skenario lamanya. Model C1.5 masih memakai satu utusan, aturan lama, dan `checkpoint.c15` melalui `--c15`. C1.4 tetap tersedia lewat `--c14` dan `checkpoint.c10`. Kampanye C1.6 memakai `checkpoint.c16`; mode piksel juga dipisahkan ke slot ini.
- Kedua rute seed tetap tersedia; C1.6 menambah pengurus balai dan kontak sosial **hanya** dalam skenario revisi 7. Kargo Reed Hall, Stonebank, pengangkut Qinghe, dan kantung obat pendamping tetap berbeda.

## Cara mencoba setelah Rust tersedia

```bash
cargo run --manifest-path apps/sealed_playable/Cargo.toml --bin sealed_agency
```

Tombol ASCII tetap `wasd`, `n`, `talk`, `invite`, `save`, `load`, `lang`, `factions`, dan `factionlog`. Tambahan: `pledge`, `aidlodge`, `renounce`, `politicslog`. Mode `--pixel` memasang tombol **4/5/6** untuk sumpah, bantuan Stonebank, dan pembatalan sumpah. Skenario dapat diulang dengan `seed 1` atau `seed 2` **sebelum giliran pertama**.

## Review dan matriks penerimaan

| Gate | Bukti yang ditulis | Status |
| --- | --- | --- |
| 2 rute | Tes menunggu 240 giliran dan mengharuskan pembelian, petisi, serta pengiriman Reed Hall | Belum dieksekusi |
| Konservasi barang/koin | Pembelian = pengiriman + muatan pengurus; ledger kedua faksi cocok dengan saldo pasar | Belum dieksekusi |
| Replay | Rev. 7 disimpan, dipulihkan, dilanjutkan 90 giliran; world setara | Belum dieksekusi |
| Isolasi lama | Rev. 6 menolak aksi baru *sebelum* tick dan mempertahankan checkpoint | Belum dieksekusi |
| Pilihan pemain | Sumpah butuh perkenalan, menghabiskan herba, melepaskan rank; bantuan dan renounce tercatat | Belum dieksekusi |
| Pendamping | Dukungan faksi diperlukan sebelum mengundang utusan/pengurus; pembatalan sumpah memutus perjanjian | Belum dieksekusi |
| Jam pasar | Pengurus tidak membeli saat malam atau kas kosong | Belum dieksekusi |
| Antrean chokepoint | Fixture penjaga memperlihatkan satu celah tidak bisa dilewati saat ditempati, pulih setelah ia pergi | Belum dieksekusi |
| Privasi UI | `politicslog` hanya menyajikan kejadian yang terlihat dan tidak memajukan waktu | Belum dieksekusi |

### Temuan dan batas saat ini

1. **Gerbang sempit tetap masalah desain terbuka.** Perbaikan fixture C1.5 dipertahankan; tes baru menegaskan bahwa satu aktor benar-benar dapat menutup celah, bukan otomatis menghilang. Mekanik meminta jalan, bertukar posisi, dan resolusi kemacetan banyak agen **belum** diimplementasikan. Hindari mengklaim bug navigasi telah selesai.
2. **Kepercayaan faksi bukan sekadar angka hiasan:** nilai dukungan dipakai untuk membuka pendamping rival dan pengurus Hall. Nilai ini belum menjadi sistem diplomasi regional yang menyebarkan berita secara bertahap ke semua pemimpin.
3. **Belum ada perang antarsekte atau serangan NPC-vs-NPC otomatis.** Ini adalah kontestasi logistik dan komitmen sosial yang dapat dimainkan. Pilihan faksi dapat mengubah aliansi pendamping dan pelayanan guru; event politik diamati secara lokal, bukan broadcast dunia.
4. **Belum ada jalur warisan lintas generasi** setelah pemain gugur; dunia dan kampanye tetap persisten sesuai keputusan Persistent World & Legacy. Stage F (regional LOD dan populasi luas) jangan dimulai sampai konflik ini lolos gate dan dibenchmark.
5. **Toolchain lokal sesi implementasi belum mempunyai `cargo`, `rustc`, maupun `rustfmt`.** Semua tes dan baris UI baru harus dieksekusi di mesin Rust sebelum klaim PASS. Tidak ada push atau merge ke GitHub.

## Saran gate kompilasi

1. `cargo fmt --all --check` dan `cargo test --lib living::` dari root.
2. `cargo test --manifest-path apps/sealed_playable/Cargo.toml --bin sealed_agency` dan tes ASCII library.
3. Clippy strict untuk `sealed_sim` dan `sealed_playable`; jalankan skrip seed parity, `--c14`, `--c15`, `--pixel`, save/reload.
4. Play manusia minimal lima menit pada kedua rute, kemudian amati 240 giliran dengan laporan harga, kemacetan, saldo kas, pemasok kuil, laporan sosial, dan pemulihan checkpoint.
5. Cocokkan checksum revisi 6 terhadap ZIP C1.5 yang sudah diuji di Windows, bukan hanya terhadap codec dalam sumber C1.6.
