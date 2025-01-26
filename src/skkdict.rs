use std::{
    collections::HashMap,
    error::Error,
    fs,
    io::{Read, Write},
    path::Path,
    sync::LazyLock,
};

const JISYO_L_URL: &str =
    "https://github.com/skk-dev/dict/raw/refs/heads/master/SKK-JISYO.L.unannotated";

static SUFFIXES: LazyLock<HashMap<&str, Vec<&str>>> = LazyLock::new(|| {
    HashMap::from([
        ("a", vec!["あ"]),
        ("b", vec!["ば", "び", "ぶ", "べ", "ぼ"]),
        // ("c", vec!["ち"]),
        ("c", vec![]),
        ("d", vec!["だ", "ぢ", "づ", "で", "ど"]),
        ("e", vec!["え"]),
        ("g", vec!["が", "ぎ", "ぐ", "げ", "ご"]),
        ("h", vec!["は", "ひ", "ふ", "へ", "ほ"]),
        ("i", vec!["い"]),
        ("j", vec!["じ"]),
        ("k", vec!["か", "き", "く", "け", "こ"]),
        ("m", vec!["ま", "み", "む", "め", "も"]),
        ("n", vec!["な", "に", "ぬ", "ね", "の", "ん"]),
        ("o", vec!["お"]),
        ("p", vec!["ぱ", "ぴ", "ぷ", "ぺ", "ぽ"]),
        ("r", vec!["ら", "り", "る", "れ", "ろ"]),
        ("s", vec!["さ", "し", "す", "せ", "そ"]),
        ("t", vec!["た", "ち", "つ", "て", "と"]),
        ("u", vec!["う"]),
        ("w", vec!["わ", "うぃ", "を", "うぇ"]),
        ("y", vec!["や", "ぃ", "ゆ", "いぇ", "よ"]),
        ("z", vec!["ざ", "じ", "ず", "ぜ", "ぞ"]),
    ])
});

/// Fetch the latest SKK-JISYO.L.unannotated from GitHub
///
/// # Returns
///
/// Contents of SKK-JISYO converted to utf-8.
pub async fn fetch_dict() -> reqwest::Result<String> {
    let response = reqwest::get(JISYO_L_URL).await?;
    response.text_with_charset("euc-jp").await
}

/// Writes the specified dictionary contents to the file at the given path.
///
/// # Arguments
///
/// * `path` - The path where the dictionary will be written.
/// * `dict` - The dictionary contents to be written.
///
/// # Returns
///
/// A Result that indicates whether the write operation was successful or not.
pub fn write_dict(path: &str, dict: &str) -> std::io::Result<()> {
    // FIXME: unwrap
    fs::create_dir_all(Path::new(path).parent().unwrap())?;
    let mut file = fs::File::create(path)?;
    file.write_all(dict.as_bytes())?;
    Ok(())
}

/// Reads the contents of the dictionary from the specified file path.
///
/// # Arguments
///
/// * `path` - The path to the dictionary file to be read.
///
/// # Returns
///
/// The dictionary contents as a String in file.
pub fn read_dict(path: &str) -> std::io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

/// Prepares the dictionary by attempting to read from the specified path.
///
/// If the dictionary file does not exist,
/// it fetches the latest version from GitHub
/// and writes it to the specified path.
///
/// # Arguments
///
/// * `path` - The path to the dictionary file.
///
/// # Returns
///
/// Contents of the dictionary if successful.
pub async fn prepare_dict(path: &str) -> Result<String, Box<dyn Error>> {
    match read_dict(path) {
        Ok(dict) => Ok(dict),
        Err(_) => {
            let dict = fetch_dict().await?;
            write_dict(path, &dict)?;
            Ok(dict)
        }
    }
}

/// Parses a line of the skk dictionary.
///
/// # Arguments
///
/// * `line` - The line to be parsed.
///
/// # Returns
///
/// * `Some((kana, kanjis))`: A tuple containing the kana and
///     kanjis' [`Vec`] within the entry.
/// * `None`: If the line is not an entry.
fn parse_skkdict_line(line: &str) -> Option<(String, Vec<String>)> {
    if line.starts_with(";;") {
        return None;
    }
    if line.is_empty() {
        return None;
    }

    let (kana, kanjis) = line.split_once(" ")?;
    if kana.starts_with(">") || kana.ends_with(">") {
        return None;
    }
    let kanjis_vec = kanjis
        .strip_prefix("/")?
        .strip_suffix("/")?
        .split("/")
        .map(|s| s.to_string())
        .collect::<Vec<String>>();

    Some((kana.to_string(), kanjis_vec))
}

/// Retrieves the suffixes associated with the given kana.
///
/// # Arguments
///
/// * `kana` - The kana string for which suffixes are to be retrieved.
///
/// # Returns
///
/// * `Some((kana_body, suffix_kanas))`: A tuple containing the kana without suffixes
///   and available additional kanas associated with the suffixes.
/// * `None`: If the kana does not have any associated suffixes.
fn get_suffixes(kana: &str) -> Option<(String, Vec<String>)> {
    let last_char = kana.chars().last()?.to_string();
    let suffix_kanas = SUFFIXES
        .get(last_char.as_str())?
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    Some((kana.strip_suffix(&last_char)?.to_string(), suffix_kanas))
}

/// Collects the possible readings from a line of the SKK dictionary.
///
/// # Arguments
///
/// * `line` - The line from which readings are to be collected.
///
/// # Returns
///
/// * `Some(vec![(kanji, kana)])`: [`Vec`] of tuples containing the kanji and corresponding kana.
/// * `None`: If the line is not an entry.
fn collect_skk_readings_from_line(line: &str) -> Option<Vec<(String, String)>> {
    let (kana, kanjis_vec) = parse_skkdict_line(line)?;

    // (kana without suffix, suffix kanas) if the last character of kana is a suffix
    let suffix_entry: Option<(String, Vec<String>)> = get_suffixes(&kana);
    let result = match suffix_entry {
        Some((ref kana_base, ref suffixes)) => kanjis_vec
            .into_iter()
            .flat_map(|kanji| {
                suffixes
                    .iter()
                    .map(|suffix| {
                        (
                            format!("{}{}", kanji, suffix),
                            format!("{}{}", kana_base, suffix),
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect(),
        None => kanjis_vec
            .into_iter()
            .map(|kanji| (kanji.clone(), kana.clone()))
            .collect(),
    };
    Some(result)
}

/// Converts the skk dictionary contents to a trie.
///
/// # Arguments
///
/// * `dict` - The dictionary contents to be converted.
///
/// # Returns
///
/// A trie containing the dictionary contents.
/// The trie key is the word with kanji.
/// The trie value is a vector of possible kanas from the kanji.
pub fn skkdict_to_trie(dict: &str) -> trie_rs::map::Trie<u8, Vec<String>> {
    // readings[kanji] = {kana1, kana2,...}
    let readings: HashMap<String, Vec<String>> = {
        let mut k = HashMap::<String, Vec<String>>::new();
        for line in dict.lines() {
            if let Some(r) = collect_skk_readings_from_line(line) {
                for (kanji, kana) in r {
                    k.entry(kanji).or_default().push(kana);
                }
            }
        }
        k
    };

    let builder = {
        let mut b = trie_rs::map::TrieBuilder::<u8, Vec<String>>::new();
        for (kanji, kanas) in readings {
            let mut k = kanas.clone();
            k.sort();
            b.insert(kanji.bytes(), k);
        }
        b
    };
    builder.build()
}

#[cfg(test)]
mod tests {
    use tokio::sync::OnceCell;

    use super::*;
    use crate::RAW_DICT_PATH;

    static DICT: OnceCell<trie_rs::map::Trie<u8, Vec<String>>> = OnceCell::const_new();
    async fn get_dict() -> trie_rs::map::Trie<u8, Vec<String>> {
        DICT.get_or_init(|| async {
            let sdict = prepare_dict(RAW_DICT_PATH).await.unwrap();
            skkdict_to_trie(&sdict)
        })
        .await
        .clone()
    }

    #[tokio::test]
    async fn dict_same_kanji_test() {
        let dict = get_dict().await;
        assert_eq!(
            dict.exact_match("擲ち".to_string()),
            Some(&vec!["なぐち".to_string(), "なげうち".to_string()])
        );
    }

    #[tokio::test]
    async fn dict_predictive_test() {
        let dict = get_dict().await;
        assert_eq!(
            dict.predictive_search::<String, _>("這い上")
                .collect::<Vec<_>>(),
            vec![
                ("這い上が".to_string(), &vec!["はいあが".to_string()]),
                ("這い上ぎ".to_string(), &vec!["はいあぎ".to_string()]),
                ("這い上ぐ".to_string(), &vec!["はいあぐ".to_string()]),
                ("這い上げ".to_string(), &vec!["はいあげ".to_string()]),
                ("這い上ご".to_string(), &vec!["はいあご".to_string()]),
                ("這い上た".to_string(), &vec!["はいのぼた".to_string()]),
                ("這い上ち".to_string(), &vec!["はいのぼち".to_string()]),
                ("這い上つ".to_string(), &vec!["はいのぼつ".to_string()]),
                ("這い上て".to_string(), &vec!["はいのぼて".to_string()]),
                ("這い上と".to_string(), &vec!["はいのぼと".to_string()]),
                ("這い上な".to_string(), &vec!["はいのぼな".to_string()]),
                ("這い上に".to_string(), &vec!["はいのぼに".to_string()]),
                ("這い上ぬ".to_string(), &vec!["はいのぼぬ".to_string()]),
                ("這い上ね".to_string(), &vec!["はいのぼね".to_string()]),
                ("這い上の".to_string(), &vec!["はいのぼの".to_string()]),
                ("這い上ら".to_string(), &vec!["はいのぼら".to_string()]),
                ("這い上り".to_string(), &vec!["はいのぼり".to_string()]),
                ("這い上る".to_string(), &vec!["はいのぼる".to_string()]),
                ("這い上れ".to_string(), &vec!["はいのぼれ".to_string()]),
                ("這い上ろ".to_string(), &vec!["はいのぼろ".to_string()]),
                ("這い上ん".to_string(), &vec!["はいのぼん".to_string()])
            ]
        );
    }

    #[tokio::test]
    async fn dict_common_prefix_test() {
        let dict = get_dict().await;
        assert_eq!(
            dict.common_prefix_search::<String, _>("安全保障理事会")
                .collect::<Vec<_>>(),
            vec![
                (
                    "安".to_string(),
                    &vec![
                        "あ".to_string(),
                        "あん".to_string(),
                        "やす".to_string(),
                        "やすし".to_string()
                    ]
                ),
                ("安全".to_string(), &vec!["あんぜん".to_string()]),
                (
                    "安全保障".to_string(),
                    &vec!["あんぜんほしょう".to_string()]
                ),
                (
                    "安全保障理事会".to_string(),
                    &vec!["あんぜんほしょうりじかい".to_string()]
                ),
            ]
        );
    }
}
