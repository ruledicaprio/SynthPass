//! #574 (PR 4b): `select_line1` on the OCR text of seven public specimens.
//!
//! `line1_selection.rs` pins the rule on constructed zones. This file pins what it
//! does on real pages: each text is the page the real-specimen bench captured for
//! that specimen (`provider-bench --dump-ocr-passes`, the acceptance capture of
//! #608), pinned whole. The expected verdicts are the ones the bench's `control`
//! and `on` replays of that capture reported.
//!
//! - Somalia 2023 and Djibouti 2019: the accepted name field is not well formed,
//!   and exactly one distinct line of the page passes every check. The proposal
//!   is the printed zone.
//! - Dominican Republic 2020: the accepted issuer is not in the country table, so
//!   the name field is left alone.
//! - Slovakia 2005, Serbia 2012, Somaliland 2023 and Uzbekistan 2013: the accepted
//!   name field is well formed, so it is kept.

use mrz::{
    find_and_parse, select_line1, Format, Line1Selection, Line1Unresolved, Line1Verdict, MrzData,
    ParseOptions,
};

/// The read `find_and_parse` accepts from `text`; every case here starts from a
/// checksum-valid TD3 read.
fn accepted(text: &str) -> MrzData {
    let data = find_and_parse(text).expect("the page parses");
    assert_eq!(data.format, Format::Td3);
    assert!(data.valid(), "the accepted read is checksum-valid");
    data
}

fn select(text: &str, accepted: &MrzData) -> Line1Selection {
    select_line1(text, accepted, &ParseOptions::default())
}

/// The proposal, after checking it differs from `incumbent` only in the name
/// fields and line 1.
fn proposal(selection: Line1Selection, incumbent: &MrzData) -> MrzData {
    let data = match selection.verdict {
        Line1Verdict::Proposed(data) => data,
        other => panic!("expected a proposal, got {other:?}"),
    };
    let mut probe = data.clone();
    probe.surname = incumbent.surname.clone();
    probe.given_names = incumbent.given_names.clone();
    probe.mrz_lines = incumbent.mrz_lines.clone();
    assert_eq!(&probe, incumbent, "only the names and line 1 change");
    data
}

// --- Pages ------------------------------------------------------------------

/// Somalia passport 2023 (TD3). The accepted line 1 has a `Z` among its fillers; three
/// lines of the page, read by the general pass and two retry passes, hold the same
/// well-formed line 1.
const SOMALIA_2023: &str = r##"1
JLesalnsee
Dau/Country Code
pSOM PO1447848 
Magaca/ /Name
ABDIAZIZ MOHAMUD IBRAHIM 
NID: 29502021985678
TN
STUDENT
Meesha laga Bixiwev/YK /Place of lestie
MOGADISHU
Meesha Dhalesbada/X.l.K /Place of
Birth
Kafilsia laga Bixivev/ oll o /lssuing Authority
SOMALI GOVERNMENT
Taariikhda uu Dbacayo/ ss)l b /Date of Expiry Saxiixa Oofka/i Holder's Signature
15 Dec 2028
JAMHUURIYADDA SOOMAALIYA
BAASABOOR il AESPORT
SOMALI REPUBLIC
Nonal esill /Type
Magaca Hooyada/3l l /Mother'e Name
1
ASLI YUSUF MOHAMUD
Jinsivada/  /Nationality
SOMALI 
Taaiildda Dhalashada/ Xll b /Date of Birth
02 Feb 1995
Lab Dheddig/ uwoll /Gender
MALE 
Taariikhda la Bixyey/ bell &b /Date of Issue
16 Dec 2023
PUBLIO
1
LH
Fneeunation
LASANOD
Shaqada/
BEPD
P<SOMIBRAHIM<<ABDIAZIZ<MOHAMUD<<<<<<<<<<<<<<
PO14478489S0M9502024M28121512950202198567888
P<SOMIBRAHIM<ABDIAZIZ<MOHAMUD<<<<<<<<<<<<<<
PO14478489S0M9502024M28121512950202198567888
P<SOMIBRAHIMC<ABDIAZIZ<MOHAMUD<<<<<<<<<<<<<<<
PO1447848950M9502024M28121512950202198567888
ABDIAZIZMOHAMUDIBRAHIM
NID29502021985678PBLG
IAMHUURIYADDASOOMAALIYA
P<SOMIBRAHIM<<ABDIAZIZ<MOHAMUD<<<<<<<<<<<<<<
PO14478489S0M9502024M28121512950202198567888
PSOMIBRAHIMABDIAZIZ<MOHAMUD<<<<<<<<<<<<<<
PO14478489S0M9502024M28121512950202198567888
PSSOMIBRAHIM<<ABDIAZIZ<MOHAMUD<<<<<<<<<<<<<<
IPO14478489S0M9502024M28121512950202198567888
P<SOMIBRAHIM<ABDIAZIZ<MOHAMUD<<<<<<<<<<<<<<
PO14478489S0M9502024M28121512950202198567888
PSSOMIBRAHIM<<ABDIAZIZ<MOHAMUD<<<<<<<<<<<<<<
PO1447848950M9502024M28121512950202198567888
P<SOMIBRAHIM<<ABDIAZIZ<MOHAMUD<<<<<<<<<<<<<<
PO14478489S0M9502024M28121512950202198567888
P<SOMIBRAHIM<<ABDIAZIZ<MOHAMUD<<<<<<<Z<<<<<<<
PO14478489S0M9502024M28121512950202198567888
P<SOMIBRAHIM<ABDIAZIZ<MOHAMUD<<<<<<<<<<<<<<
P014478489S0M9502024M28121512950202198567888"##;

/// Djibouti passport 2019 (TD3). The accepted line 1 reads an `S` for the filler between two
/// names, and letters among the trailing fillers; one line of the page holds a well-formed
/// line 1.
const DJIBOUTI_2019: &str = r##"CoLAPDRAANARATV
SSEPORT
Ay
REPUBLIQUE DE DJIBOUTI
DJI
HASSAN FARID RAGUE
B
19RF45329
DUBOUT
26071991
2019
GP
Defce Nationale
2024
P<DJIHASSAN<FARID<RAGUE<<<<<<<<<<<<<<<<<<<<<
RF153208D J19107267M2408046<<<<<<<<<<<<<<04
VCCTRM
1.70
SABIEH
MINIST?RE DE L'INT?RIEUR
Barron
RoUt
MACKTI
MINISTEREDELINTERIEUR
P<DJIHASSANSFARID<RAGUEK<C<<<<<KK<<<<<<<<<
18RF153208DJ19107267M2408046<<<<<<<<<<<<<<04"##;

/// Dominican Republic passport 2020 (TD3). The accepted line 1 reads the issuer as `DOR`,
/// which is not in the country table.
const DOMINICAN_2020: &str = r##"H
?
19
*
8
7
98
8
68 E88
3
e
$OB
8R1R8
BEE
S
885258
E
B3
R?P?BLICA DOMINICANA Dominican Republic
Ihoorror nodde pea oiunty Ooort ort Feset 
P  DOMRD5893175
D
MATOS VALENZUELA
JANIBEL
DOMIN?CANA
A
10 MAY/MAY 2000402-1011454-8
nigr
PSANTO DOMINGO, RD
ae Atono
28 FEB/FEB2020 DISTRITO NAC.
d
28 FEB/FEB 2026t?.
yamibek
PD58931752D0M0005108F260228040210114548<<<48
PASAPORTE
Passoort
TO
U
Fe
B3A
PSDOMMATOS<VALENZUELA<<JANIBEL<<<<<<<<<<<<<<
WPASSOORDONRD5893175
10MAYZMAY2000F40210114548
128FEBFEB2020DISTRITOANAC
PSDOMMATOS<VALENZUELAS<JANIBELSS<<<<<<<<SS<<
RD58931752D0M0005108F2602280402101
70MAYRAY200030210114548
PEDORRATOS<VALENZUELA<<JANIBELT<SS<<<<<E<<<<
RD5893175200N0005108F260228040210114548<<<48"##;

/// Slovakia passport 2005 (TD3). The accepted name field is well formed.
const SLOVAKIA_2005: &str = r##"SLOVENSK? REPUBLIKA / SLOVAK REPUBLIC_/ REPUBLIQUE SLOVAQUE
J PEPE CTRCOTECODE
S?K
EZ/SXD6 EU0A
SPECIMEN
AODS
VZOR
PPIANOST NTONGTYNEONFE
HSOPRSI RSPAT GSERSAT
P0000000
CESTOUNY PAS
PASSPORT
PASSEPORT
M
SK
SULMSACE DE DE BRTHINNERENARAEC
1.11.1911
LEEX/SDE MCSTONAROOEHA/FLACE O"OFRI/U?DE HAGEANCE
BANSK? BYSTRICA
YAIE L JELSRENCE
DATEOFDPRY IRGTE REORTICH
01.04,2015
ADOCK
H
POMENEAYEN No
111111/1111
SSSEUEY/FUTOTE
PAPPE
BRATISLAVA
PORSFRECAHOERE SNATLE SNTRG E TIY
pecuion
01.04.2005
PKSVKSPECIMEN<<VZOR<<<<<<<<<<<<<<<<<<<
PO00000O<5SVK1111112M1501043<<<<<<<<<<<<<<00
B
Ko essssh SAsoeS Aoe
NRATNOCRDNEOEDRRYERTEDEORTEN
POOPSERATBOAHOJDER3STERATUESYNATRGDETI
P00000005SVK1111112M1501043<<<<3<00
FADEDNCEATCEREMSSCAESMR
PSSVKSPECIMEN<VZOR<<<<<<<<<<<<<<<<<<<<<<<<<
P0000000<5SVK1111112M1501043<<<<<<<<<<<<<<00
2287388888888880122202000"##;

/// Serbia passport 2012 (TD3). The accepted name field is well formed.
const SERBIA_2012: &str = r##"WREWELmnNT2 LSECENo
PE??????KA CPBIJA REPUBLIC OE SERBIA
Tun/Type /iype  Kon/Code LCode
PSRB
fpesnme/ Strname Nom
TEST
MMe Givan name /Prenom S^
MILICA
IpxKaBrbaRCTBO Nationalty.Natonalit?
SRPSKO
Jaryi poheipa Datesoffidh Date deriatx
29 021968 
on.sexvere .. edo nnaa  onhe ss ware obbito.L au denaiccande
ZIF BEOGRAD REPUBLIKA SRBIA
RoebMBamuue Hndotexdende Famkdall
NOVI BEOGRAD BEOGRAB
EUTOPWA BBT
Manar ou Autheno kAMD
MUPR SRBIJE PUZA GRAD BEOGRAD/ATN
BaryMwenadatba ae h ao dae detelyfanreno MorangSdnane/Sicnatre
Iinc
Mwhdd Jert
REPUBLIQUE DESERBE
5p. nacowa/ Passport No: du passer
000000000
?ACoU
PASSPORT
PASSEPORT 
T
MbE RersonaN o  barsnon
22902968000000
" 10 10 2012
Baxkn no Dote ofexouwDate dka.
10 10 2022
60666660600
P<SRBTEST<MILICA<<<KK<K<<<<K<<<<<<<<<<
NO
XXXX
0000000000SRB6802295F22101042902968000000<40"##;

/// Somaliland passport 2023 (TD3, a non-ISO issuer). The accepted name field is well
/// formed.
const SOMALILAND_2023: &str = r##"De svroe eet n foab
ic d
aasaboorkan
SAR
This passport' is valid
walba
waa
llau
31
couintries
kara
R
Ta
Baasaboor ambar
Passport No.
P00040973
JAMHUURIYADDA SOMALILAND
Mebca Type Astaanta Daka/ Country Code Baassboor tambor  Passpot
Astganta
RSL PO0040973
Macaca Awoowwa Sumwme
AvocITa
SOMALILAND SPECIMEN
Madaca Givon Mamiea
MUSTAFE
Mationalty
SOMALILANDER
hshada / Date
Taarikhda Dhalashada/ Date of Bith
01 JAN 2000
Genobta
Jieai Ser GGoo?ta Ohalashada Plece of Birth
HARGEISA
Tanrchoa a Rieivf Date Assue Awooda Rixigta 
DDste 
ofue
19 OCT 2023 SOMALIL AND IMMIGRATION
Dhncako / 0ote
Fonrikha uu Ohacayo Oate of EXpiy
OCT 2028
REPUBLIC OF SOMALILAND
Raanaboo
BAASABOOR
PASSPORT
Jinalyadda 
lachada
esU
Autnority
Holder's
Samnexn
Seha
Sionatore
PSRSLSOMALILAND<SPECIMEN<<MUSTAFE<<<<<<<<<<<
P000409734RSL0001018M28101790000033<<<<<<<42"##;

/// Uzbekistan passport 2013 (TD3). The accepted name field is well formed.
const UZBEKISTAN_2013: &str = r##"YC
Se
FOVRINROYEV
RI
ABDURAXIMJON
?UOABOYEVICH
TUG'LGAN SANASTUG'ILGANJOY
25 03 1973 FARG'ONA TUMANI
'??ZBEK
TOMONIDAN BERILGAN
ARGONA VILOYATI FARG'ONA
TUMANI IIB
?:
FUTIOO
O'ZBEKISTON RESPUBLIKASI
1::
"
ERKAK ?:
S
?
B
Apaut
SHAXSIY IMZO HOL DER'S SIGNATURE 
O'ZBEKISTON RESPUBLIKASI/ REPUBLIC OF UZBEKISTAN
TURITYPE DAVLAT KOOICOUNTRYODE PASPORT RAGAMI/PASSPORT
PASSPORT I7 3456559
UZB AA
PASPORT 
FAMILYASSURNAME
URINBOEV
IVEN
NAMES
ABDURAKHIM.ION
FUGAROUIGI /NATIONAL
UZBEKISTAN
TUGILGAN SANASDATE OF BIRTH
25 03 197
JNSISEX TUGLGAN JOVIACE
ERGANA REGION
RERGAN SANASIDATE OF ISSUE PERSONALLASHTIRISH ARCANAUTORMT
30 11 2013 STATE PERSONALIZATION
AMAL OILISH MUDDATIDATE OF EXPIRY CENTRE
OHLISH
29 11 2023
URTH
FXPIRY
PSUZBURINBOEV<<ABDURAKHIMJON<<<<<<<<<<<<<<<<
AA34565591UZB7303250M23112963250373427012788"##;

// --- Proposed ---------------------------------------------------------------

#[test]
fn somalia_2023_proposes_the_printed_zone() {
    let incumbent = accepted(SOMALIA_2023);
    assert_eq!(
        incumbent.mrz_lines,
        "P<SOMIBRAHIM<<ABDIAZIZ<MOHAMUD<<<<<<Z<<<<<<<\n\
         P014478489SOM9502024M28121512950202198567888"
    );
    let selection = select(SOMALIA_2023, &incumbent);
    assert_eq!((selection.eligible, selection.distinct), (3, 1));
    let data = proposal(selection, &incumbent);
    assert_eq!(
        data.mrz_lines,
        "P<SOMIBRAHIM<<ABDIAZIZ<MOHAMUD<<<<<<<<<<<<<<\n\
         P014478489SOM9502024M28121512950202198567888"
    );
    let printed = find_and_parse(&data.mrz_lines).expect("the printed zone parses");
    assert_eq!(
        (data.surname.as_str(), data.given_names.as_str()),
        (printed.surname.as_str(), printed.given_names.as_str())
    );
}

#[test]
fn djibouti_2019_proposes_the_printed_zone() {
    let incumbent = accepted(DJIBOUTI_2019);
    assert_eq!(
        incumbent.mrz_lines,
        "P<DJIHASSANSFARID<RAGUEK<C<<<<<KK<<<<<<<<<<<\n\
         18RF153208DJI9107267M2408046<<<<<<<<<<<<<<04"
    );
    let selection = select(DJIBOUTI_2019, &incumbent);
    assert_eq!((selection.eligible, selection.distinct), (1, 1));
    let data = proposal(selection, &incumbent);
    assert_eq!(
        data.mrz_lines,
        "P<DJIHASSAN<FARID<RAGUE<<<<<<<<<<<<<<<<<<<<<\n\
         18RF153208DJI9107267M2408046<<<<<<<<<<<<<<04"
    );
    let printed = find_and_parse(&data.mrz_lines).expect("the printed zone parses");
    assert_eq!(
        (data.surname.as_str(), data.given_names.as_str()),
        (printed.surname.as_str(), printed.given_names.as_str())
    );
}

// --- Unresolved -------------------------------------------------------------

#[test]
fn dominican_2020_with_an_unresolved_issuer_is_left_alone() {
    let incumbent = accepted(DOMINICAN_2020);
    assert_eq!(
        incumbent.mrz_lines,
        "PEDORRATOS<VALENZUELA<<JANIBELT<SS<<<<<E<<<<\n\
         RD58931752OON0005108F260228040210114548<<<48"
    );
    let selection = select(DOMINICAN_2020, &incumbent);
    assert_eq!(
        selection.verdict,
        Line1Verdict::Unresolved(Line1Unresolved::IssuerUnresolved)
    );
    assert_eq!((selection.eligible, selection.distinct), (0, 0));
}

// --- Kept -------------------------------------------------------------------

/// Each page's accepted zone, whose name field is already well formed.
const KEPT: [(&str, &str); 4] = [
    (
        SLOVAKIA_2005,
        "PSSVKSPECIMEN<VZOR<<<<<<<<<<<<<<<<<<<<<<<<<<\n\
         P0000000<5SVK1111112M1501043<<<<<<<<<<<<<<00",
    ),
    (
        SERBIA_2012,
        "P<SRBTEST<MILICA<<<<<<<<<<<<<<<<<<<<<<<<<<<<\n\
         0000000000SRB6802295F22101042902968000000<40",
    ),
    (
        SOMALILAND_2023,
        "PSRSLSOMALILAND<SPECIMEN<<MUSTAFE<<<<<<<<<<<\n\
         P000409734RSL0001018M28101790000033<<<<<<<42",
    ),
    (
        UZBEKISTAN_2013,
        "PSUZBURINBOEV<<ABDURAKHIMJON<<<<<<<<<<<<<<<<\n\
         AA34565591UZB7303250M23112963250373427012788",
    ),
];

#[test]
fn a_well_formed_accepted_name_field_is_kept() {
    for (text, zone) in KEPT {
        let incumbent = accepted(text);
        assert_eq!(incumbent.mrz_lines, zone);
        let selection = select(text, &incumbent);
        assert_eq!(selection.verdict, Line1Verdict::Kept, "{zone}");
        assert_eq!((selection.eligible, selection.distinct), (0, 0), "{zone}");
    }
}
