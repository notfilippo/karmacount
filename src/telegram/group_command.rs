use std::{collections::HashSet, sync::Arc};

use bincode::{deserialize, serialize};
use sled::Db;
use teloxide::{
    adaptors::DefaultParseMode,
    requests::{Requester, ResponseResult},
    types::{Message, MessageId, UserId},
    utils::command::BotCommands,
    Bot,
};

use crate::{db, error::Error};

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum GroupCommand {
    #[command(description = "display leaderboard.")]
    Leaderboard,
}

async fn handler(
    bot: DefaultParseMode<Bot>,
    db: Arc<Db>,
    msg: Message,
    cmd: GroupCommand,
) -> Result<(), Error> {
    match cmd {
        GroupCommand::Leaderboard => {
            let db_members = db.open_tree(db::TREE_MEMBERS)?;
            let members = db_members
                .get(msg.chat.id.to_string())?
                .map_or(Ok(HashSet::<UserId>::new()), |bytes| deserialize(&bytes))?;

            if members.len() == 0 {
                let text = "<i>There are no members with karma in this group.</i>";
                bot.send_message(msg.chat.id, text).await?;
                return Ok(());
            }

            let db_karma = db.open_tree(db::TREE_KARMA)?;
            let db_last_message = db.open_tree(db::TREE_LAST_MESSAGE)?;

            let mut leaderboard = members
                .iter()
                .map(|id| {
                    let karma = db_karma
                        .get(id.to_string())?
                        .map_or(Ok(db::DEFAULT_KARMA), |bytes| deserialize(&bytes))?;
                    Ok((id, karma))
                })
                .collect::<Result<Vec<_>, Error>>()?;

            leaderboard.sort_by(|(_, a), (_, b)| b.cmp(a));

            let mut text = String::new();
            for (i, (id, karma)) in leaderboard.iter().enumerate() {
                let chat = bot.get_chat(**id).await?;
                let name = chat
                    .username()
                    .map(|username| format!("@{}", username))
                    .unwrap_or(chat.first_name().unwrap_or("N/A").to_string());
                let mention = format!("<a href=\"tg://user?id={}\">{}</a>", id, name);
                text.push_str(&format!("{}. {} : {}", i + 1, mention, karma));
                text.push('\n');
            }

            let last_message_key = format!("{}-leaderboard", msg.chat.id);
            let last_message: Option<MessageId> = db_last_message
                .get(&last_message_key)?
                .map_or(Ok(None), |bytes| deserialize(&bytes).map(|id| Some(id)))?;

            if let Some(last_message) = last_message {
                bot.delete_message(msg.chat.id, last_message).await.ok();
            }

            let message = bot.send_message(msg.chat.id, text).await?;
            db_last_message.insert(&last_message_key, serialize(&message.id)?)?;
        }
    };

    Ok(())
}

pub async fn command_handler(
    bot: DefaultParseMode<Bot>,
    db: Arc<Db>,
    msg: Message,
    cmd: GroupCommand,
) -> ResponseResult<()> {
    match handler(bot, db, msg, cmd).await {
        Ok(_) => Ok(()),
        Err(e) => match e {
            Error::DatabaseError(err) => {
                log::error!("Database error: {}", err);
                Ok(())
            }
            Error::DecodingError(err) => {
                log::error!("Decoding error: {}", err);
                Ok(())
            }
            Error::TelegramError(err) => Err(err),
        },
    }
}
