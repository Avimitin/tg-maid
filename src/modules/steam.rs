use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename = "@profile")]
pub struct SteamUserInfo {
    #[serde(rename = "steamID64")]
    pub id: u64,
    #[serde(rename = "steamID")]
    pub name: String,
    #[serde(rename = "stateMessage")]
    pub state: String,
    #[serde(rename = "avatarFull")]
    pub avatar_url: String,
}

impl SteamUserInfo {
    pub fn try_from_str(s: &str) -> anyhow::Result<Self> {
        let profile: Self = quick_xml::de::from_str(s)?;

        Ok(Self {
            name: profile.name.trim().to_string(),
            state: profile.state.trim().to_string(),
            avatar_url: profile.avatar_url.trim().to_string(),
            ..profile
        })
    }
}

#[test]
fn test_xml_deserialize() {
    let pseudo_steam_profile = r#"
<profile>
    <steamID64>1145141919810</steamID64>
    <steamID>
        <![CDATA[ 先辈 ]]>
    </steamID>
    <onlineState>online</onlineState>
    <stateMessage>
        <![CDATA[ Online ]]>
    </stateMessage>
    <avatarFull>
        <![CDATA[ https://avatars.cloudflare.steamstatic.com/1145141919810_full.jpg ]]>
    </avatarFull>
</profile>
    "#;
    let user = SteamUserInfo::try_from_str(pseudo_steam_profile).unwrap();

    assert_eq!(user.id, 1145141919810);
    assert_eq!(user.name, "先辈");
    assert_eq!(user.state, "Online");
    assert_eq!(
        user.avatar_url,
        "https://avatars.cloudflare.steamstatic.com/1145141919810_full.jpg"
    );
}
