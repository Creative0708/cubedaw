pub struct ResourceKey {
    str: String,
    divider: usize,
}

// yes this is taken from minecraft. i like how the keys look ok
impl ResourceKey {
    pub fn new(namespace: &str, key: &str) -> Self {
        assert!(
            namespace.len() > 0 && namespace.is_ascii(),
            "ResourceKey namespace has to be a non-empty ascii string"
        );
        assert!(
            key.len() > 0 && key.is_ascii(),
            "ResourceKey key has to be a non-empty ascii string"
        );

        for b in namespace.bytes().chain(key.bytes()) {
            assert!(matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'_' | b'.'))
        }

        Self {
            str: format!("{namespace}:{key}"),
            divider: namespace.len(),
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
