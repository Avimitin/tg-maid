use regex::Regex;
use std::collections::HashMap;

lazy_static::lazy_static! {
    pub static ref EAT_PATTEN: regex::Regex = Regex::new(r#"^[^/]*吃什么.*$"#).unwrap();
}

#[test]
fn test_eat_pattern() {
    let pending = vec![
        "吃什么？",
        "吃什么",
        "今天吃什么",
        "今天吃什么？",
        "吃什么好呢？",
        "今天吃什么好呢？",
    ];

    for (i, p) in pending.iter().enumerate() {
        dbg!(i);
        assert!(EAT_PATTEN.is_match(p));
    }

    assert!(!EAT_PATTEN.is_match("/吃什么"))
}

trait IntoRespond {
    fn to_respond(&self, cap: &regex::Captures) -> String;
}

impl<'a> IntoRespond for &'a str {
    fn to_respond(&self, _: &regex::Captures) -> String {
        self.to_string()
    }
}

impl<'a> IntoRespond for &'a String {
    fn to_respond(&self, _: &regex::Captures) -> String {
        self.to_string()
    }
}

impl<F> IntoRespond for F
where
    F: Fn(&'_ regex::Captures) -> String,
{
    fn to_respond(&self, cap: &regex::Captures) -> String {
        self(cap)
    }
}

/// Represent something like "pattern": |words, index| Some(String::new())
type RulePair = HashMap<&'static str, Box<dyn Fn(&[&str], usize) -> Option<String> + Send + Sync>>;

pub struct Patterns {
    jieba: jieba_rs::Jieba,
    store: RulePair,
}

impl std::fmt::Debug for Patterns {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Pattern store: HashMap<...>")
    }
}

impl Patterns {
    pub fn generate() -> Self {
        let mut store: RulePair = HashMap::new();

        macro_rules! rule {
            ($key:literal, $callback: expr) => {
                store.insert($key, Box::new($callback))
            };
        }

        macro_rules! shuffle {
            ($a:expr, $b:expr) => {
                if rand::random() {
                    $a.to_string()
                } else {
                    $b.to_string()
                }
            };
        }

        rule!("吃", |words, i| {
            if words.len() - 1 <= i {
                return None;
            }
            let Some(j) = words.iter().position(|w| w == &"还是") else { return None; };
            if words.len() - 1 <= j {
                return None;
            }

            let mut choices: Vec<_> = words[i + 1..]
                .split(|w| [",", "，", " ", "还是"].contains(w))
                .filter(|slice| !slice.is_empty())
                .collect();
            let selected = choices.swap_remove(rand::random::<usize>() % choices.len());
            Some(selected.concat())
        });

        rule!("能", |words, i| {
            if words.len() - 1 < i {
                return None;
            }

            words
                .iter()
                .skip(i + 1)
                .find(|x| **x == "吗")
                .map(|_| shuffle!("能！", "不能！"))
        });

        rule!("不", |words, i| {
            if words.len() - 1 < i || i < 1 {
                return None;
            }

            let mut tailing = false;

            // if end with "不？"
            if let Some(last) = words.last() {
                if ["?", "？"].contains(last) && words.len() - 2 == i {
                    tailing = true
                }
            }

            // if end with only ”不“ itself
            if !tailing && words.len() - 1 == i {
                tailing = true
            }

            if !tailing && words[i - 1] != words[i + 1] {
                return None;
            }

            if rand::random() {
                Some(format!("不{} !", words[i - 1]))
            } else {
                Some(words[i - 1].to_string())
            }
        });

        rule!("是不是", |_, _| { Some(shuffle!("是的", "不是")) });

        rule!("是", |words, i| {
            let mut tails = words.iter().skip(i + 1);

            let result = tails.next().and_then(|s| match *s {
                "吧" => Some(shuffle!("还真是", "那还真不是")),
                "吗" => Some(shuffle!("是的", "不是")),
                _ => None,
            });

            if result.is_some() {
                return result;
            }

            if let Some(j) = words.iter().position(|x| x == &"还是") {
                return {
                    if words.len() - 1 <= j {
                        None
                    } else {
                        Some(shuffle!(
                            format!("是{}", words[i + 1..j].concat()),
                            format!("是{}", words[j + 1..].concat())
                        ))
                    }
                };
            }

            None
        });

        Self {
            store,
            jieba: Self::new_jieba(),
        }
    }

    fn new_jieba() -> jieba_rs::Jieba {
        let mut jb = jieba_rs::Jieba::new();
        jb.add_word("资磁", None, None);
        jb
    }

    pub fn try_match(&self, t: &str) -> Option<String> {
        let words = self.jieba.cut(t, true);
        for (key, callback) in &self.store {
            if let Some(i) = words.iter().position(|x| x == key) {
                return callback(&words, i);
            }
        }

        None
    }
}

#[test]
fn test_match() {
    let pat = Patterns::generate();

    let repl = pat.try_match("玄学了是吧");
    assert!(repl.is_some());
    assert!(repl.unwrap().contains('是'));

    let repl = pat.try_match("到底买不买新手机呢");
    assert!(repl.is_some());
    assert!(repl.unwrap().contains('买'));

    let repl = pat.try_match("吃饭不？");
    assert!(repl.is_some());
    assert!(repl.unwrap().contains("吃饭"));

    let repl = pat.try_match("资磁不资磁？");
    assert!(repl.is_some());
    assert!(repl.unwrap().contains("资磁"));

    let repl = pat.try_match("是向左好还是向右好呢");
    assert!(repl.is_some());
    let repl = repl.unwrap();
    assert!(repl.starts_with('是'));
    assert!(!repl.contains("还是"));

    let expect = ["麦当劳", "肯德基", "必胜客"];
    let repl = pat.try_match("吃麦当劳，肯德基，还是必胜客");
    assert!(repl.is_some());
    let repl = repl.unwrap();
    dbg!(&repl);
    assert!(expect.contains(&repl.as_str()));

    let repl = pat.try_match("吃麦当劳 肯德基 还是必胜客");
    assert!(repl.is_some());
    let repl = repl.unwrap();
    dbg!(&repl);
    assert!(expect.contains(&repl.as_str()));

    let repl = pat.try_match("吃麦当劳，肯德基，还是必胜客");
    assert!(repl.is_some());
    let repl = repl.unwrap();
    dbg!(&repl);
    assert!(expect.contains(&repl.as_str()));

    let repl = pat.try_match("纠结吃麻辣香锅还是热干面");
    assert!(repl.is_some());
    let repl = repl.unwrap();
    dbg!(&repl);
    assert!(["麻辣香锅", "热干面"].contains(&repl.as_str()));
}
