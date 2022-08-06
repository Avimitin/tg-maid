use std::collections::HashMap;

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

        macro_rules! randomize {
            ($one:literal, $second:literal) => {
                if rand::random() {
                    String::from($one)
                } else {
                    String::from($second)
                }
            };

            ($a:expr, $b:expr) => {
                if rand::random() {
                    $a.to_string()
                } else {
                    $b.to_string()
                }
            };
        }

        rule!("不", |words, i| {
            if words.len() - 1 < i {
                return None;
            }

            let mut tailing = false;

            if let Some(last) = words.last() {
                if ["?", "？"].contains(last) {
                    tailing = true
                }
            }

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

        rule!("是不是", |_, _| { Some(randomize!("是的", "不是")) });

        rule!("是", |words, i| {
            let mut tails = words.iter().skip(i + 1);
            if let Some(s) = tails.next().and_then(|s| {
                if s == &"吧" {
                    Some(randomize!("是的", "不是"))
                } else {
                    None
                }
            }) {
                Some(s)
            } else if tails.clone().any(|x| x == &"吗") {
                Some(randomize!("是的", "不是"))
            } else if let Some(j) = words.iter().position(|x| x == &"还是") {
                if words.len() - 1 <= j {
                    return None;
                }

                Some(randomize!(
                    format!("是{}", words[i + 1..j].concat()),
                    format!("是{}", words[j + 1..].concat())
                ))
            } else {
                None
            }
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
    assert!(repl.unwrap().contains(&"买"));

    let repl = pat.try_match("吃饭不？");
    assert!(repl.is_some());
    assert!(repl.unwrap().contains(&"吃饭"));

    let repl = pat.try_match("资磁不资磁？");
    assert!(repl.is_some());
    assert!(repl.unwrap().contains(&"资磁"));

    let repl = pat.try_match("是向左好还是向右好呢");
    assert!(repl.is_some());
    assert!(repl.unwrap().starts_with('是'));
}
