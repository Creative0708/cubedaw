use std::sync::Arc;

use crate::Id;

#[derive(Clone)]
pub struct ResourceKey {
    str: Arc<str>,
    divider: usize,
}

// yes this is stolen from minecraft. i like how the keys look ok
impl ResourceKey {
    // TODO make this return Option<Self> or Result<Self, _>
    pub fn new_split(namespace: &str, key: &str) -> Self {
        assert!(
            !namespace.is_empty() && namespace.is_ascii(),
            "ResourceKey namespace has to be a non-empty ascii string"
        );
        assert!(
            !key.is_empty() && key.is_ascii(),
            "ResourceKey key has to be a non-empty ascii string"
        );

        for b in namespace.bytes().chain(key.bytes()) {
            assert!(matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'_' | b'.'))
        }

        Self {
            str: Arc::from(format!("{namespace}:{key}")),
            divider: namespace.len(),
        }
    }
    // TODO make this return Option<Self> or Result<Self, _>
    pub fn new(str: &str) -> Self {
        assert!(!str.is_empty() && str.is_ascii());

        let mut divider = None;
        for (i, b) in str.bytes().enumerate() {
            if b == b':' {
                if divider.is_some() {
                    panic!("duplicate : in ResourceKey");
                }
                divider = Some(i);
            } else {
                assert!(
                    matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'_' | b'.'),
                    "invalid byte: {:?}",
                    b as char,
                );
            }
        }

        let Some(divider) = divider else {
            panic!("no : in ResourceKey");
        };

        Self {
            divider,
            str: Arc::from(str),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.str
    }
    pub fn namespace(&self) -> &str {
        &self.str[..self.divider]
    }
    pub fn key(&self) -> &str {
        &self.str[self.divider + 1..]
    }

    pub fn id(&self) -> Id<Self> {
        Id::new(self)
    }
}

impl std::fmt::Debug for ResourceKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.str, f)
    }
}

impl std::hash::Hash for ResourceKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.str.hash(state)
    }
}

impl PartialEq for ResourceKey {
    fn eq(&self, other: &Self) -> bool {
        self.str == other.str
    }
}
impl Eq for ResourceKey {}
