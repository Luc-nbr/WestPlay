/**
 * What the Maps app says instead of the map when the player plays on a map WestPlay doesn't allow (the host
 * tells: a map by HVT, Heavy Vehicles Turkey), in the game's language: every language the game has, by
 * its own codes (`locale/<code>` in its archives). HVT and WestPlay are names, the same in every language.
 * The game shows Silesian players Polish (pl_si), and so does this.
 */
export const HVT_MAP_MESSAGES: Record<string, string> = {
  bg_bg: 'Картите от HVT не са разрешени в Dashly. Свържете се с Dashly, за да научите повече.',
  ca_es: "Els mapes d'HVT no estan permesos a Dashly. Contacta amb Dashly per saber-ne més.",
  cs_cz: 'Mapy od HVT nejsou ve Dashly povoleny. Pro více informací kontaktujte Dashly.',
  da_dk: 'Kort fra HVT er ikke tilladt i Dashly. Kontakt Dashly for at få mere at vide.',
  de_de: 'Karten von HVT sind in Dashly nicht erlaubt. Kontaktiere Dashly, um mehr zu erfahren.',
  el_gr: 'Οι χάρτες της HVT δεν επιτρέπονται στο Dashly. Επικοινωνήστε με το Dashly για να μάθετε περισσότερα.',
  en_gb: 'Maps by HVT are not allowed in Dashly. Contact Dashly to learn more.',
  en_us: 'Maps by HVT are not allowed in Dashly. Contact Dashly to learn more.',
  es_es: 'Los mapas de HVT no están permitidos en Dashly. Contacta con Dashly para saber más.',
  es_la: 'Los mapas de HVT no están permitidos en Dashly. Comunícate con Dashly para saber más.',
  et_ee: 'HVT kaardid ei ole Dashlys lubatud. Lisateabe saamiseks võta ühendust Dashlyga.',
  eu_es: 'HVTren mapak ez daude baimenduta Dashlyn. Jarri harremanetan Dashlyrekin gehiago jakiteko.',
  fi_fi: 'HVT:n kartat eivät ole sallittuja Dashlyssa. Ota yhteyttä Dashlyhin saadaksesi lisätietoja.',
  fr_ca: 'Les cartes de HVT ne sont pas permises dans Dashly. Communiquez avec Dashly pour en savoir plus.',
  fr_fr: 'Les cartes de HVT ne sont pas autorisées dans Dashly. Contactez Dashly pour en savoir plus.',
  gl_es: 'Os mapas de HVT non están permitidos en Dashly. Contacta con Dashly para saber máis.',
  hr_hr: 'Karte od HVT-a nisu dopuštene u Dashlyu. Za više informacija obratite se Dashlyu.',
  hu_hu: 'A HVT térképei nem engedélyezettek a Dashlyben. További információért lépj kapcsolatba a Dashlyjel.',
  it_it: 'Le mappe di HVT non sono consentite in Dashly. Contatta Dashly per saperne di più.',
  ja_jp: 'HVTのマップはDashlyでは使用できません。詳しくはDashlyにお問い合わせください。',
  ka_ge: 'HVT-ის რუკები Dashly-ში დაშვებული არ არის. მეტის გასაგებად დაუკავშირდით Dashly-ს.',
  ko_kr: 'HVT의 지도는 Dashly에서 허용되지 않습니다. 자세한 내용은 Dashly에 문의하세요.',
  lt_lt: 'HVT žemėlapiai Dashly neleidžiami. Norėdami sužinoti daugiau, susisiekite su Dashly.',
  lv_lv: 'HVT kartes Dashly nav atļautas. Lai uzzinātu vairāk, sazinieties ar Dashly.',
  mk_mk: 'Мапите од HVT не се дозволени во Dashly. Контактирајте го Dashly за да дознаете повеќе.',
  nl_nl: 'Kaarten van HVT zijn niet toegestaan in Dashly. Neem contact op met Dashly voor meer informatie.',
  no_no: 'Kart fra HVT er ikke tillatt i Dashly. Kontakt Dashly for å finne ut mer.',
  pl_pl: 'Mapy od HVT nie są dozwolone w Dashly. Skontaktuj się z Dashly, aby dowiedzieć się więcej.',
  pl_si: 'Mapy od HVT nie są dozwolone w Dashly. Skontaktuj się z Dashly, aby dowiedzieć się więcej.',
  pt_br: 'Mapas da HVT não são permitidos no Dashly. Entre em contato com o Dashly para saber mais.',
  pt_pt: 'Os mapas da HVT não são permitidos no Dashly. Contacte o Dashly para saber mais.',
  ro_ro: 'Hărțile HVT nu sunt permise în Dashly. Contactează Dashly pentru a afla mai multe.',
  ru_ru: 'Карты от HVT не разрешены в Dashly. Свяжитесь с Dashly, чтобы узнать больше.',
  sk_sk: 'Mapy od HVT nie sú vo Dashly povolené. Pre viac informácií kontaktujte Dashly.',
  sl_sl: 'Zemljevidi HVT v Dashlyu niso dovoljeni. Za več informacij se obrnite na Dashly.',
  sr_sp: 'Mape od HVT-a nisu dozvoljene u Dashly-u. Kontaktirajte Dashly da saznate više.',
  sr_sr: 'Мапе од HVT-а нису дозвољене у Dashly-у. Контактирајте Dashly да сазнате више.',
  sv_se: 'Kartor från HVT är inte tillåtna i Dashly. Kontakta Dashly för att få veta mer.',
  tr_tr: "HVT haritalarına Dashly'de izin verilmiyor. Daha fazla bilgi için Dashly ile iletişime geçin.",
  uk_uk: "Карти від HVT не дозволені у Dashly. Зв'яжіться з Dashly, щоб дізнатися більше.",
  vi_vn: 'Bản đồ của HVT không được phép sử dụng trong Dashly. Liên hệ Dashly để tìm hiểu thêm.',
  zh_cn: 'Dashly 不允许使用 HVT 的地图。如需了解详情，请联系 Dashly。',
  zh_tw: 'Dashly 不允許使用 HVT 的地圖。如需瞭解詳情，請聯絡 Dashly。',
}

/** The message in the game's `language`, or English for one not listed (a mod's). */
export function hvtMapMessage(language: string | null): string {
  return HVT_MAP_MESSAGES[language ?? ''] ?? HVT_MAP_MESSAGES.en_us
}
