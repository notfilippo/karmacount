mod db;
mod error;
mod telegram;

use std::{env, sync::Arc};

use teloxide::{prelude::*, types::ParseMode};

use crate::telegram::{group_command, message, root_command, user_command};

const DB_PATH: &str = "data";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    pretty_env_logger::init();
    log::info!("Starting karma bot...");

    let root = env::var("ROOT").map_or(UserId(0), |root| UserId(root.parse::<u64>().unwrap_or(0)));
    let token = env::var("TOKEN").expect("TOKEN must be set");

    let bot = Bot::new(token).parse_mode(ParseMode::Html);

    let db = Arc::new(sled::open(DB_PATH)?);

    let handler = Update::filter_message()
        .branch(
            dptree::filter(|root: UserId, msg: Message| {
                msg.from().map(|user| user.id == root).unwrap_or(false)
            })
            .filter_command::<root_command::RootCommand>()
            .endpoint(root_command::command_handler),
        )
        .branch(
            dptree::filter(|msg: Message| msg.chat.is_private())
                .filter_command::<user_command::UserCommand>()
                .endpoint(user_command::command_handler),
        )
        .branch(
            dptree::filter(|msg: Message| msg.chat.is_group() || msg.chat.is_supergroup())
                .filter_command::<group_command::GroupCommand>()
                .endpoint(group_command::command_handler),
        )
        .branch(
            dptree::filter(|msg: Message| msg.chat.is_group() || msg.chat.is_supergroup())
                .endpoint(message::message_handler),
        );

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![root, db])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}
