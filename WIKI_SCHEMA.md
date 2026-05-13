# CLAUDE.md — LLM Wiki Şeması

Bu dosya, Claude Code'un bu wiki deposunu nasıl yöneteceğini tanımlar. Tüm işlemlerde bu kurallara uy.

---

## Proje Yapısı

```
my-project/
├── CLAUDE.md              # Bu dosya — şema ve kurallar
├── raw/                   # Ham kaynaklar (SALT OKUNUR — asla değiştirme)
│   ├── articles/
│   ├── books/
│   ├── notes/
│   ├── transcripts/
│   └── assets/            # Görseller ve medya dosyaları
├── wiki/                  # LLM tarafından üretilen ve sürdürülen sayfalar
│   ├── index.md           # İçerik kataloğu
│   ├── log.md             # Kronolojik işlem günlüğü
│   ├── overview.md        # Wikinin üst düzey sentezi
│   ├── entities/          # Kişiler, kurumlar, ürünler vb.
│   ├── concepts/          # Fikirler, teoriler, çerçeveler
│   ├── sources/           # Her ham kaynak için özet sayfası
│   ├── comparisons/       # Karşılaştırma ve analiz sayfaları
│   └── queries/           # Sorgulamalardan doğan kalıcı sayfalar
└── tools/                 # İsteğe bağlı yardımcı betikler
```

---

## Temel Kurallar

1. **`raw/` dizini dokunulmazdır.** Asla oluşturma, düzenleme veya silme yapma. Yalnızca oku.
2. **`wiki/` dizininin tamamı senin sorumluluğundadır.** Tüm sayfaları sen oluşturur, güncellersin ve bakımını yaparsın.
3. **Her değişiklikte `index.md` ve `log.md` dosyalarını güncelle.**
4. **Obsidian uyumluluğunu koru.** Bağlantılarda `[[page-name]]` wiki-link formatını kullan.
5. **Dosya adları İngilizce, küçük harf, tire ile ayrılmış slug formatında olmalıdır.** Örnek: `ai-ethics.md`, `openai.md`, `reinforcement-learning.md`
6. **Her sayfa YAML ön bilgisiyle başlamalıdır** (aşağıdaki şablonlara bak).
7. **Çelişkileri gizleme, açıkça işaretle.** Kaynaklar arasında tutarsızlık varsa belirt.

---

## Sayfa Şablonları

### Kaynak Özeti (`wiki/sources/`)

Her özümsenen ham kaynak için bir tane oluştur.

```markdown
---
title: "Kaynak Başlığı"
type: source
source_file: "raw/articles/file-name.md"
author: "Yazar Adı"
date: 2026-05-04
created: 2026-05-04
updated: 2026-05-04
tags: [tag1, tag2]
---

# Kaynak Başlığı

## Özet
Kaynağın 3-5 cümlelik özeti.

## Temel Çıkarımlar
- Birinci çıkarım
- İkinci çıkarım
- Üçüncü çıkarım

## Detaylı Notlar
Kaynağın ayrıntılı açıklaması. Önemli argümanlar, veriler ve sonuçlar burada yer alır.

## Bağlantılar
- İlgili varlıklar: [[entity-name]]
- İlgili kavramlar: [[concept-name]]
- Çelişen/destekleyen kaynaklar: [[other-source]]

## Alıntılar
Doğrudan referans verilmek istenen önemli cümleler, sayfa numarasıyla birlikte.
```

### Varlık Sayfası (`wiki/entities/`)

Kişiler, kurumlar, ürünler, yerler vb. için.

```markdown
---
title: "Varlık Adı"
type: entity
category: person | organization | product | place | other
created: 2026-05-04
updated: 2026-05-04
source_count: 3
tags: [tag1, tag2]
---

# Varlık Adı

## Tanım
Varlığın kısa tanımı.

## Temel Bilgiler
Varlık hakkında biriken bilgilerin sentezi. Birden fazla kaynaktan derlenen tutarlı bir anlatı.

## Kaynaklarda Geçişi
- [[source-1]]: Bu kaynakta nasıl geçtiği
- [[source-2]]: Bu kaynakta nasıl geçtiği

## İlişkiler
- [[other-entity]]: İlişkinin tanımı
- [[concept]]: Bu kavramla bağlantısı
```

### Kavram Sayfası (`wiki/concepts/`)

Fikirler, teoriler, çerçeveler, teknik terimler için.

```markdown
---
title: "Kavram Adı"
type: concept
created: 2026-05-04
updated: 2026-05-04
source_count: 2
tags: [tag1, tag2]
---

# Kavram Adı

## Tanım
Kavramın net ve özlü tanımı.

## Detaylı Açıklama
Kavramın derinlemesine açıklaması. Farklı kaynakların bu kavramı nasıl ele aldığının sentezi.

## Örnekler
Kavramın somut örneklerle açıklanması.

## İlişkili Kavramlar
- [[related-concept-1]]: Nasıl ilişkili olduğu
- [[related-concept-2]]: Farkları ve benzerlikleri

## Kaynaklar
- [[source-1]]: Bu kaynaktaki perspektif
- [[source-2]]: Farklı veya destekleyici bakış açısı
```

### Karşılaştırma Sayfası (`wiki/comparisons/`)

```markdown
---
title: "X ve Y Karşılaştırması"
type: comparison
created: 2026-05-04
updated: 2026-05-04
tags: [tag1, tag2]
---

# X ve Y Karşılaştırması

## Özet
Karşılaştırmanın kısa özeti ve temel sonuç.

## Karşılaştırma Tablosu

| Boyut         | X                  | Y                  |
|---------------|--------------------|--------------------|
| Boyut 1       | ...                | ...                |
| Boyut 2       | ...                | ...                |

## Detaylı Analiz
Her boyutun ayrıntılı açıklaması.

## Sonuç
Genel değerlendirme ve bağlama göre hangisinin ne zaman tercih edilebileceği.

## Kaynaklar
- [[source-1]], [[source-2]]
```

---

## İşlemler

### 1. Özümseme (Ingest)

Kullanıcı `raw/` dizinine yeni bir kaynak eklediğinde ve işlenmesini istediğinde:

**Adım adım akış:**

1. Kaynağı baştan sona oku.
2. Kullanıcıyla temel çıkarımları tartış. Neyin önemli olduğunu sor.
3. `wiki/sources/` altında bir özet sayfası oluştur.
4. Wiki genelinde etkilenen sayfaları belirle:
   - Yeni veya mevcut **varlık sayfalarını** güncelle/oluştur.
   - Yeni veya mevcut **kavram sayfalarını** güncelle/oluştur.
   - `wiki/overview.md` dosyasını gerekiyorsa güncelle.
5. Çapraz referansları ekle — hem yeni sayfalardan eskilere, hem eskilerden yeniye.
6. **Çelişki kontrolü yap.** Yeni kaynak mevcut sayfalarla çelişiyorsa:
   - İlgili sayfalarda çelişkiyi `> ⚠️ **Çelişki:**` bloğuyla işaretle.
   - Her iki tarafın kaynağını belirt.
7. `wiki/index.md` dosyasını güncelle.
8. `wiki/log.md` dosyasına giriş ekle.
9. Yapılan değişikliklerin özetini kullanıcıya sun.

**Toplu özümseme:** Kullanıcı birden fazla kaynağı aynı anda işletmek isterse, her birini sırayla özümse. Ancak çapraz referansları ve çelişki kontrollerini tüm yeni kaynaklar arasında da yap.

### 2. Sorgulama (Query)

Kullanıcı wikiye bir soru sorduğunda:

1. Önce `wiki/index.md` dosyasını oku — ilgili sayfaları belirle.
2. Belirlenen sayfaları oku ve sentezle.
3. Yanıtı kaynakları ile birlikte sun (`[[page-name]]` bağlantılarıyla).
4. **Değerli yanıtları wikiye kaydet.** Eğer yanıt:
   - Bir karşılaştırma içeriyorsa → `wiki/comparisons/` altına kaydet.
   - Yeni bir sentez veya analiz içeriyorsa → `wiki/queries/` altına kaydet.
   - Kullanıcıya sor: "Bu yanıtı wikiye kaydetmemi ister misin?"
5. Kaydedilen yanıtları `index.md` ve `log.md`'ye ekle.

### 3. Denetleme (Lint)

Kullanıcı wiki sağlık kontrolü istediğinde:

Aşağıdaki kontrolleri yap ve bir rapor sun:

- **Çelişkiler:** Farklı sayfalarda birbiriyle çelişen iddialar.
- **Eski iddialar:** Daha yeni kaynakların geçersiz kıldığı bilgiler.
- **Yetim sayfalar:** Hiçbir başka sayfadan bağlantı almayan sayfalar.
- **Eksik sayfalar:** Birden fazla sayfada bahsedilen ama kendi sayfası olmayan kavram veya varlıklar.
- **Eksik çapraz referanslar:** Birbiriyle ilişkili olması gereken ama bağlantısız sayfalar.
- **Veri boşlukları:** Web aramasıyla doldurulabilecek eksik bilgiler.
- **Öneriler:** Araştırılacak yeni sorular ve aranacak yeni kaynaklar.

Raporu kullanıcıya sunduktan sonra, onay alarak düzeltmeleri uygula.

---

## index.md Formatı

```markdown
# Wiki Dizini

Son güncelleme: 2026-05-04

## Kaynaklar
- [[source-slug]] — Kısa açıklama (Yazar, Tarih) [tag1, tag2]

## Varlıklar
- [[entity-slug]] — Kısa tanım (Kategori) [source_count: N]

## Kavramlar
- [[concept-slug]] — Kısa tanım [source_count: N]

## Karşılaştırmalar
- [[comparison-slug]] — Neyi neyle karşılaştırdığı

## Sorgular
- [[query-slug]] — Sorgunun kısa açıklaması (Tarih)
```

Her özümseme, sorgulama veya denetleme sonrası bu dosyayı güncelle.

---

## log.md Formatı

```markdown
# Wiki Günlüğü

## [2026-05-04] ingest | Kaynak Başlığı
- Oluşturulan sayfalar: [[page-1]], [[page-2]]
- Güncellenen sayfalar: [[page-3]], [[page-4]]
- Tespit edilen çelişkiler: [[page-5]] ↔ [[page-6]]

## [2026-05-04] query | Soru özeti
- Yanıt kaydedildi: [[query-slug]]

## [2026-05-04] lint
- Bulunan sorunlar: 3 yetim sayfa, 1 çelişki, 2 eksik çapraz referans
- Düzeltilen sorunlar: 2 yetim sayfa bağlandı, 1 çelişki işaretlendi
```

Her giriş `## [YYYY-MM-DD] operation-type | Açıklama` formatında olmalıdır. Bu formatı koru — `grep "^## \[" wiki/log.md | tail -10` komutuyla son girişlere erişilebilir.

---

## Çapraz Referans Kuralları

1. **Çift yönlü bağlantı:** A sayfası B'ye bağlanıyorsa, B de A'ya bağlanmalıdır.
2. **Bağlam içinde bağlantı:** Bağlantıları yalnızca listelemek yerine, metin akışı içinde doğal şekilde kullan.
3. **İlk geçişte bağlantı ver:** Bir sayfada bir varlık veya kavramdan ilk bahsedildiğinde wiki-link ekle. Aynı sayfada tekrar bağlantı verme.
4. **Varlık → Kaynak bağlantısı:** Bir varlık sayfası, o varlığın geçtiği tüm kaynak sayfalarına bağlantı içermelidir.
5. **Kaynak → Varlık/Kavram bağlantısı:** Bir kaynak özeti, kaynakta geçen tüm önemli varlık ve kavramlara bağlantı içermelidir.

---

## Yazım Kuralları

- **Dil:** İçerik Türkçe yazılır. Teknik terimlerin Türkçe karşılığını kullan; yaygın kabul görmüş İngilizce terimler parantez içinde verilebilir (ör. "büyük dil modeli (LLM)").
- **Dosya ve dizin adları:** Her zaman İngilizce. Dosya adları, dizin adları, slug'lar, YAML anahtarları ve wiki-link referansları İngilizce olmalıdır. İçerik (başlıklar, açıklamalar, analizler) Türkçe kalır.
- **Ton:** Ansiklopedik, nesnel, kısa ve öz. Gereksiz tekrardan kaçın.
- **Uzunluk:** Kaynak özetleri 300-800 kelime. Varlık ve kavram sayfaları biriken bilgiye göre büyür.
- **Çelişki gösterimi:** `> ⚠️ **Çelişki:**` bloğu kullan, her iki tarafın kaynağını belirt.
- **Belirsizlik gösterimi:** `> ❓ **Belirsiz:**` bloğu kullan, neyin eksik olduğunu belirt.
- **Tarihler:** ISO 8601 formatında (YYYY-MM-DD).

---

## Genel Bakış Sayfası (`wiki/overview.md`)

Bu sayfa, wikinin tamamının üst düzey sentezini içerir. Şunları kapsar:

- Wikinin kapsamı ve amacı
- Ana temalar ve bunlar arasındaki ilişkiler
- Gelişen tezler ve sonuçlar
- Bilinen boşluklar ve açık sorular

Her önemli özümseme sonrası bu sayfayı gözden geçir ve gerekirse güncelle.

---

## Başlatma

Eğer `wiki/` dizini henüz yoksa, ilk çalıştırmada şu yapıyı oluştur:

```bash
mkdir -p wiki/{entities,concepts,sources,comparisons,queries}
```

Boş `index.md`, `log.md` ve `overview.md` dosyalarını ilgili başlık şablonlarıyla oluştur. Ardından kullanıcıya wikinin hazır olduğunu bildir ve ilk kaynağı özümsemek isteyip istemediğini sor.

---

## Hatırlatmalar

- Bir şeyi silmeden önce düşün — güncellemek genellikle silmekten iyidir.
- Her oturumun başında `log.md`'nin son girişlerini oku; bağlamı hatırla.
- Kullanıcı yeni bir oturum açtığında, son durumu kısaca özetle.
- Kapsamlı değişikliklerden önce kullanıcıya bilgi ver ve onay al.
- Wikinin bir git deposu olduğunu unutma — kullanıcı geçmişe dönebilir.
