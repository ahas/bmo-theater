//! The subtitle formats converted to WebVTT: SRT nearly as it is, SAMI by its timed blocks and its first
//! language class, ASS to its text, WebVTT kept; and a file's bytes read in the encoding they carry.

use bmo_theater::subtitles::{convert, decode, vtt_time};
use bmo_theater::wire::SubtitleFormat;

#[test]
fn srt_converts_with_its_timestamps_and_known_markup() {
  let srt = "\u{feff}1\r\n00:00:01,500 --> 00:00:03,000\r\n<i>Hello</i> <font color=\"red\">there</font>\r\nsecond line\r\n\r\n2\r\n00:00:04,000 --> 00:00:05,250\r\nBye\r\n";
  let converted = convert(SubtitleFormat::Srt, srt).unwrap();
  assert_eq!(converted.cues, 2);
  assert_eq!(
    converted.vtt,
    "WEBVTT\n\n00:00:01.500 --> 00:00:03.000\n<i>Hello</i> there\nsecond line\n\n00:00:04.000 --> 00:00:05.250\nBye\n"
  );
}

#[test]
fn sami_converts_by_its_blocks_each_lasting_until_the_next() {
  let smi = r#"<SAMI>
<HEAD><STYLE TYPE="text/css"><!--
P { margin-left: 8pt; }
.KRCC { Name: Korean; lang: ko-KR; }
.ENCC { Name: English; lang: en-US; }
--></STYLE></HEAD>
<BODY>
<SYNC Start=1000><P Class=KRCC>First<br>line two
<SYNC Start=1000><P Class=ENCC>First in English
<SYNC Start=2500><P Class=KRCC>&nbsp;
<SYNC Start=3000><P Class=KRCC>Third &amp; last
</BODY></SAMI>"#;
  let converted = convert(SubtitleFormat::Smi, smi).unwrap();
  assert_eq!(converted.cues, 2);
  assert_eq!(
    converted.vtt,
    "WEBVTT\n\n00:00:01.000 --> 00:00:02.500\nFirst\nline two\n\n00:00:03.000 --> 00:00:07.000\nThird &amp; last\n"
  );
}

#[test]
fn ass_converts_to_its_text_alone() {
  let ass = "[Script Info]\nTitle: test\n\n[V4+ Styles]\nFormat: Name, Fontname\nStyle: Default,Arial\n\n[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\nDialogue: 0,0:00:02.00,0:00:04.50,Default,,0,0,0,,{\\an8\\b1}Top, with a comma\\Nand a break\nComment: 0,0:00:03.00,0:00:04.00,Default,,0,0,0,,not shown\nDialogue: 0,0:00:01.00,0:00:01.80,Default,,0,0,0,,First<\n";
  let converted = convert(SubtitleFormat::Ass, ass).unwrap();
  assert_eq!(converted.cues, 2);
  assert_eq!(
    converted.vtt,
    "WEBVTT\n\n00:00:01.000 --> 00:00:01.800\nFirst&lt;\n\n00:00:02.000 --> 00:00:04.500\nTop, with a comma\nand a break\n"
  );
}

#[test]
fn webvtt_is_kept_and_a_file_without_cues_is_refused() {
  let vtt = "WEBVTT\n\nNOTE a note\n\nintro\n00:01.000 --> 00:02.000\nHi\n";
  let converted = convert(SubtitleFormat::Vtt, vtt).unwrap();
  assert_eq!(converted.vtt, "WEBVTT\n\n00:00:01.000 --> 00:00:02.000\nHi\n");
  assert!(convert(SubtitleFormat::Srt, "no cues at all").is_err());
  assert!(convert(SubtitleFormat::Smi, "<SAMI><BODY></BODY></SAMI>").is_err());
}

#[test]
fn bytes_are_read_in_the_encoding_they_carry() {
  let words = "\u{c548}\u{b155}";
  let (euc_kr, _, _) = encoding_rs::EUC_KR.encode(words);
  assert_eq!(decode(&euc_kr), words);
  let mut utf16 = vec![0xFF, 0xFE];
  for unit in words.encode_utf16() {
    utf16.extend(unit.to_le_bytes());
  }
  assert_eq!(decode(&utf16), words);
  let mut marked = vec![0xEF, 0xBB, 0xBF];
  marked.extend(words.as_bytes());
  assert_eq!(decode(&marked), words);
  assert_eq!(vtt_time(3_723_004), "01:02:03.004");
}
