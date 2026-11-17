/**
 * What the Maps app says instead of the map when the player plays on a map WestPlay doesn't allow (the host
 * tells: a map by HVT, Heavy Vehicles Turkey), in the game's language: every language the game has, by
 * its own codes (`locale/<code>` in its archives). HVT and WestPlay are names, the same in every language.
 * The game shows Silesian players Polish (pl_si), and so does this.
 */
export const HVT_MAP_MESSAGES: Record<string, string> = {
  bg_bg: 'Картите от HVT не са разрешени в Euro-Hub. Свържете се с Euro-Hub, за да научите повече.',
  ca_es: "Els mapes d'HVT no estan permesos az Euro-Hub. Contacta amb Euro-Hub per saber-ne més.",
  cs_cz: 'Mapy od HVT nejsou ve Euro-Hub povoleny. Pro více informací kontaktujte Euro-Hub.',
  da_dk: 'Kort fra HVT er ikke tilladt i Euro-Hub. Kontakt Euro-Hub for at få mere at vide.',
  de_de: 'Karten von HVT sind in Euro-Hub nicht erlaubt. Kontaktiere Euro-Hub, um mehr zu erfahren.',
  el_gr: 'Οι χάρτες της HVT δεν επιτρέπονται στο Euro-Hub. Επικοινωνήστε με το Euro-Hub για να μάθετε περισσότερα.',
  en_gb: 'Maps by HVT are not allowed in Euro-Hub. Contact Euro-Hub to learn more.',
  en_us: 'Maps by HVT are not allowed in Euro-Hub. Contact Euro-Hub to learn more.',
  es_es: 'Los mapas de HVT no están permitidos en Euro-Hub. Contacta con Euro-Hub para saber más.',
  es_la: 'Los mapas de HVT no están permitidos en Euro-Hub. Comunícate con Euro-Hub para saber más.',
  et_ee: 'HVT kaardid ei ole Euro-Hubis lubatud. Lisateabe saamiseks võta ühendust Euro-Hubiga.',
  eu_es: 'HVTren mapak ez daude baimenduta Euro-Hubean. Jarri harremanetan Euro-Hubekin gehiago jakiteko.',
  fi_fi: 'HVT:n kartat eivät ole sallittuja Euro-Hubissa. Ota yhteyttä Euro-Hubiin saadaksesi lisätietoja.',
  fr_ca: 'Les cartes de HVT ne sont pas permises dans Euro-Hub. Communiquez avec Euro-Hub pour en savoir plus.',
  fr_fr: 'Les cartes de HVT ne sont pas autorisées dans Euro-Hub. Contactez Euro-Hub pour en savoir plus.',
  gl_es: 'Os mapas de HVT non están permitidos en Euro-Hub. Contacta con Euro-Hub para saber máis.',
  hr_hr: 'Karte od HVT-a nisu dopuštene u Euro-Hubu. Za više informacija obratite se Euro-Hubu.',
  hu_hu: 'A HVT térképei nem engedélyezettek az Euro-Hubban. További információért lépj kapcsolatba az Euro-Hubbal.',
  it_it: 'Le mappe di HVT non sono consentite in Euro-Hub. Contatta Euro-Hub per saperne di più.',
  ja_jp: 'HVTのマップはEuro-Hubでは使用できません。詳しくはEuro-Hubにお問い合わせください。',
  ka_ge: 'HVT-ის რუკები Euro-Hub-ში დაშვებული არ არის. მეტის გასაგებად დაუკავშირდით Euro-Hub-ს.',
  ko_kr: 'HVT의 지도는 Euro-Hub에서 허용되지 않습니다. 자세한 내용은 Euro-Hub에 문의하세요.',
  lt_lt: 'HVT žemėlapiai Euro-Hub neleidžiami. Norėdami sužinoti daugiau, susisiekite su Euro-Hub.',
  lv_lv: 'HVT kartes Euro-Hub nav atļautas. Lai uzzinātu vairāk, sazinieties ar Euro-Hub.',
  mk_mk: 'Мапите од HVT не се дозволени во Euro-Hub. Контактирајте го Euro-Hub за да дознаете повеќе.',
  nl_nl: 'Kaarten van HVT zijn niet toegestaan in Euro-Hub. Neem contact op met Euro-Hub voor meer informatie.',
  no_no: 'Kart fra HVT er ikke tillatt i Euro-Hub. Kontakt Euro-Hub for å finne ut mer.',
  pl_pl: 'Mapy od HVT nie są dozwolone w Euro-Hub. Skontaktuj się z Euro-Hub, aby dowiedzieć się więcej.',
  pl_si: 'Mapy od HVT nie są dozwolone w Euro-Hub. Skontaktuj się z Euro-Hub, aby dowiedzieć się więcej.',
  pt_br: 'Mapas da HVT não são permitidos no Euro-Hub. Entre em contato com o Euro-Hub para saber mais.',
  pt_pt: 'Os mapas da HVT não são permitidos no Euro-Hub. Contacte o Euro-Hub para saber mais.',
  ro_ro: 'Hărțile HVT nu sunt permise în Euro-Hub. Contactează Euro-Hub pentru a afla mai multe.',
  ru_ru: 'Карты от HVT не разрешены в Euro-Hub. Свяжитесь с Euro-Hub, чтобы узнать больше.',
  sk_sk: 'Mapy od HVT nie sú vo Euro-Hub povolené. Pre viac informácií kontaktujte Euro-Hub.',
  sl_sl: 'Zemljevidi HVT v Euro-Hubu niso dovoljeni. Za več informacij se obrnite na Euro-Hub.',
  sr_sp: 'Mape od HVT-a nisu dozvoljene u Euro-Hubu. Kontaktirajte Euro-Hub da saznate više.',
  sr_sr: 'Мапе од HVT-а нису дозвољене у Euro-Hub-у. Контактирајте Euro-Hub да сазнате више.',
  sv_se: 'Kartor från HVT är inte tillåtna i Euro-Hub. Kontakta Euro-Hub för att få veta mer.',
  tr_tr: "HVT haritalarına Euro-Hub'da izin verilmiyor. Daha fazla bilgi için Euro-Hub ile iletişime geçin.",
  uk_uk: "Карти від HVT не дозволені у Euro-Hub. Зв'яжіться з Euro-Hub, щоб дізнатися більше.",
  vi_vn: 'Bản đồ của HVT không được phép sử dụng trong Euro-Hub. Liên hệ Euro-Hub để tìm hiểu thêm.',
  zh_cn: 'Euro-Hub 不允许使用 HVT 的地图。如需了解详情，请联系 Euro-Hub。',
  zh_tw: 'Euro-Hub 不允許使用 HVT 的地圖。如需瞭解詳情，請聯絡 Euro-Hub。',
}

/** The message in the game's `language`, or English for one not listed (a mod's). */
export function hvtMapMessage(language: string | null): string {
  return HVT_MAP_MESSAGES[language ?? ''] ?? HVT_MAP_MESSAGES.en_us
}
