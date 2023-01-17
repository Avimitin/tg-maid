use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone, Debug)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
pub enum Command {
    #[command(description = "Display this help message")]
    Help,

    #[command(
        description = "Search exchange rate. Usage example: /exchange 1 usd cny",
        parse_with = "split"
    )]
    Exchange(f64, String, String),

    #[command(description = "Search weather. Usage example: /weather 上海")]
    Weather,

    #[command(description = "获取买家秀")]
    Mjx,

    #[command(description = "随机二次元色图")]
    Ghs,

    #[command(description = "查询 e-hentai 链接内的本子信息")]
    Eh,

    #[command(description = "获取 e-hentai 链接内的种子链接")]
    EhSeed,

    #[command(description = "收集所有内容并合并")]
    Collect,

    #[command(description = "结束收集")]
    CollectDone,

    #[command(description = "Search package information in Arch Linux Repo and AUR")]
    Pacman,

    #[command(description = "Interact with ksyx")]
    HitKsyx,

    #[command(description = "Interact with piggy")]
    CookPiggy,

    #[command(description = "Get some useful id")]
    Id,

    #[command(description = "Translate text by DeepL")]
    Translate,

    #[command(description = "Translate text by DeepL")]
    Tr,

    #[command(description = "Calculate the sex compatibility")]
    CanISexWith,

    #[command(description = "Calculate the sex compatibility")]
    Cisw,
}
