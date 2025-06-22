impl<'de> serde::Deserialize<'de> for crate::ResourceKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let str = <std::borrow::Cow<'_, str>>::deserialize(deserializer)?;
        Ok(Self::new(&str).map_err(|err| {
            serde::de::Error::custom(format!("{str:?} isn't a valid resource key: {err:?}"))
        })?)
    }
}
impl<'de> serde::Deserialize<'de> for crate::Namespace {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let str = <std::borrow::Cow<'_, str>>::deserialize(deserializer)?;
        Ok(Self::new(&str).map_err(|err| {
            serde::de::Error::custom(format!("{str:?} isn't a valid resource key: {err:?}"))
        })?)
    }
}
// impl<'de> serde::Deserialize<'de> for crate::Item {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: serde::Deserializer<'de>,
//     {
//         let str = <std::borrow::Cow<'_, str>>::deserialize(deserializer)?;
//         Ok(Self::new(&str).map_err(|err| {
//             serde::de::Error::custom(format!("{str:?} isn't a valid resource key: {err:?}"))
//         })?)
//     }
// }
