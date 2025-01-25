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

static SUFFIXES: LazyLock<HashMap<char, Vec<&str>>> = LazyLock::new(|| {
    HashMap::from([
        ('a', vec!["あ"]),
        ('b', vec!["ば", "び", "ぶ", "べ", "ぼ"]),
        // ('c', vec!["ち"]),
        ('c', vec![]),
        ('d', vec!["だ", "ぢ", "づ", "で", "ど"]),
        ('e', vec!["え"]),
        ('g', vec!["が", "ぎ", "ぐ", "げ", "ご"]),
        ('h', vec!["は", "ひ", "ふ", "へ", "ほ"]),
        ('i', vec!["い"]),
        ('j', vec!["じ"]),
        ('k', vec!["か", "き", "く", "け", "こ"]),
        ('m', vec!["ま", "み", "む", "め", "も"]),
        ('n', vec!["な", "に", "ぬ", "ね", "の", "ん"]),
        ('o', vec!["お"]),
        ('p', vec!["ぱ", "ぴ", "ぷ", "ぺ", "ぽ"]),
        ('r', vec!["ら", "り", "る", "れ", "ろ"]),
        ('s', vec!["さ", "し", "す", "せ", "そ"]),
        ('t', vec!["た", "ち", "つ", "て", "と"]),
        ('u', vec!["う"]),
        ('w', vec!["わ", "うぃ", "を", "うぇ"]),
        ('y', vec!["や", "ぃ", "ゆ", "いぇ", "よ"]),
        ('z', vec!["ざ", "じ", "ず", "ぜ", "ぞ"]),
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
    let kanas_of_kanji: HashMap<String, Vec<String>> = {
        let mut k = HashMap::<String, Vec<String>>::new();
        for line in dict.lines() {
            if line.starts_with(";;") {
                continue;
            }
            if line.is_empty() {
                continue;
            }

            let (kana, kanjis) = if let Some(x) = line.split_once(" ") {
                x
            } else {
                continue;
            };

            if kana.starts_with(">") || kana.ends_with(">") {
                continue;
            }

            let kanjis_vec = (|| kanjis.strip_prefix("/")?.strip_suffix("/"))()
                .unwrap_or_default()
                .split("/")
                .collect::<Vec<_>>();

            // (kana without suffix, suffix kanas) if the last character of kana is a suffix
            let suffix_entry: Option<(String, Vec<String>)> = (|| {
                let last_char = &kana.chars().last().unwrap();
                let suffix_kanas = SUFFIXES
                    .get(last_char)?
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>();
                Some((
                    kana.strip_suffix(&last_char.to_string())
                        .unwrap_or(kana)
                        .to_string(),
                    suffix_kanas,
                ))
            })();
            for kanji in kanjis_vec {
                match suffix_entry {
                    Some((ref kana_base, ref suffixes)) => {
                        for suffix in suffixes {
                            k.entry(format!("{}{}", &kanji, &suffix))
                                .or_default()
                                .push(format!("{}{}", &kana_base, &suffix));
                        }
                    }
                    None => {
                        k.entry(kanji.to_string())
                            .or_default()
                            .push(kana.to_string());
                    }
                }
            }
        }
        k
    };

    let builder = {
        let mut b = trie_rs::map::TrieBuilder::<u8, Vec<String>>::new();
        for (kanji, kanas) in kanas_of_kanji {
            let mut k = kanas.clone();
            k.sort();
            b.insert(kanji.bytes(), k);
        }
        b
    };
    builder.build()
}
