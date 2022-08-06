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

pub struct Patterns {
    store: HashMap<u32, (regex::Regex, Box<dyn IntoRespond + Send + Sync>)>,
}

impl std::fmt::Debug for Patterns {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Pattern store: HashMap<...>")
    }
}

impl Patterns {
    /// Prepare a set of rule for respondind when specific regex is matched.
    pub fn prepare() -> Self {
        let mut id = 0;
        let mut store: HashMap<u32, (regex::Regex, Box<dyn IntoRespond + Send + Sync>)> =
            HashMap::new();
        macro_rules! rule {
            ($regex:literal, $repl:expr) => {
                id += 1;
                store.insert(
                    id,
                    (
                        regex::Regex::new($regex)
                            .unwrap_or_else(|_| panic!("invalid regex {}", $regex)),
                        Box::new($repl),
                    ),
                )
            };
        }

        rule!("^[^/]*.*是不是", |_: &regex::Captures| {
            if rand::random() {
                "是的".to_string()
            } else {
                "不是".to_string()
            }
        });

        rule!("^[^/]*.*是.*吗", |_: &regex::Captures| {
            if rand::random() {
                "是的".to_string()
            } else {
                "不是".to_string()
            }
        });

        rule!("^[^/]*.*是吧", |_: &regex::Captures| {
            if rand::random() {
                "是的".to_string()
            } else {
                "不是".to_string()
            }
        });

        rule!("^[^/]*.*买不买", "买!");

        rule!("^[^/]*.*是(.*)还是(.*)", |cap: &regex::Captures| {
            if cap.len() < 3 {
                return "不是".to_string();
            }

            let first = &cap[1];
            let second = &cap[2];

            if rand::random() {
                format!("是{first}")
            } else {
                format!("是{second}")
            }
        });

        Self { store }
    }

    pub fn try_match(&self, t: &str) -> Option<String> {
        for (reg, rep) in self.store.values() {
            if let Some(cap) = reg.captures(t) {
                let resp = rep.to_respond(&cap);
                return Some(resp);
            }
        }

        None
    }
}

#[test]
fn test_match() {
    let pat = Patterns::prepare();

    let repl = pat.try_match("玄学了是吧");
    assert!(repl.is_some());
    assert!(repl.unwrap().contains('是'));

    let repl = pat.try_match("到底买不买新手机呢");
    assert_eq!(Some("买!".to_string()), repl);

    let repl = pat.try_match("是向左好呢还是向右好呢");
    assert!(repl.is_some());
    assert!(repl.unwrap().starts_with('是'));
}
