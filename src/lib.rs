mod skkdict;

const RAW_DICT_PATH: &str = "dict/download/SKK-JISYO.L.unannotated-utf8";

#[cfg(test)]
mod tests {
    use tokio::sync::OnceCell;

    use super::*;

    static DICT: OnceCell<trie_rs::map::Trie<u8, Vec<String>>> = OnceCell::const_new();
    async fn get_dict() -> trie_rs::map::Trie<u8, Vec<String>> {
        DICT.get_or_init(|| async {
            let sdict = skkdict::prepare_dict(RAW_DICT_PATH).await.unwrap();
            skkdict::skkdict_to_trie(&sdict)
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
