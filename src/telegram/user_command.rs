use std::sync::Arc;

use bincode::deserialize;
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use sled::Db;
use teloxide::{
    adaptors::DefaultParseMode,
    requests::{Requester, ResponseResult},
    types::Message,
    utils::command::BotCommands,
    Bot,
};

use crate::{db, error::Error};

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum UserCommand {
    #[command(description = "start a conversation with the bot.")]
    Start,
    #[command(description = "see your stats.")]
    Stats,
}

async fn handler(
    bot: DefaultParseMode<Bot>,
    db: Arc<Db>,
    msg: Message,
    cmd: UserCommand,
) -> Result<(), Error> {
    match cmd {
        UserCommand::Start | UserCommand::Stats => {
            if let Some(sender) = msg.from() {
                let db_up = db.open_tree(db::TREE_UP)?;
                let db_down = db.open_tree(db::TREE_DOWN)?;
                let db_karma = db.open_tree(db::TREE_KARMA)?;
                let db_last = db.open_tree(db::TREE_LAST)?;

                let current_last = db_last
                    .get(sender.id.to_string())?
                    .map_or(Ok(db::DEFAULT_LAST), |bytes| deserialize(&bytes))?;

                let now = Utc::now();
                let then =
                    DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(current_last, 0), Utc);

                let midnight = (then + Duration::days(1)).date().and_hms(0, 0, 0);
                let expired = now.gt(&midnight);

                let karma = db_karma
                    .get(sender.id.to_string())?
                    .map_or(Ok(db::DEFAULT_KARMA), |bytes| deserialize(&bytes))?;

                let up = match expired {
                    true => db::DEFAULT_UP,
                    false => db_up
                        .get(sender.id.to_string())?
                        .map_or(Ok(db::DEFAULT_UP), |bytes| deserialize(&bytes))?,
                };

                let down = match expired {
                    true => db::DEFAULT_DOWN,
                    false => db_down
                        .get(sender.id.to_string())?
                        .map_or(Ok(db::DEFAULT_DOWN), |bytes| deserialize(&bytes))?,
                };

                let text = format!(
                    "Your stats: \n\
                    - {} karma\n\
                    - {} + available today\n\
                    - {} - available today",
                    karma, up, down
                );
                bot.send_message(msg.chat.id, text).await?;
            }
        }
    };

    Ok(())
}

pub async fn command_handler(
    bot: DefaultParseMode<Bot>,
    db: Arc<Db>,
    msg: Message,
    cmd: UserCommand,
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
