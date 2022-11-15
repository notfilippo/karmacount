use std::sync::Arc;

use teloxide::{
    adaptors::DefaultParseMode,
    requests::{Requester, ResponseResult},
    types::Message,
    utils::command::BotCommands,
    Bot,
};

use crate::{
    business::{self, DEFAULT_DOWN, DEFAULT_UP},
    db::Store,
    error::Error,
};

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
    db: Arc<Store>,
    msg: Message,
    cmd: UserCommand,
) -> Result<(), Error> {
    match cmd {
        UserCommand::Start | UserCommand::Stats => {
            if let Some(sender) = msg.from() {
                let current_last = db.last.get_or(sender.id.to_string(), 0)?;
                let expired = business::is_assignable_karma_expired(current_last);

                let karma = db.karma.get_or(sender.id.to_string(), 0)?;

                let (up, down) = match expired {
                    true => (DEFAULT_UP, DEFAULT_DOWN),
                    false => (
                        db.up.get_or(sender.id.to_string(), DEFAULT_UP)?,
                        db.down.get_or(sender.id.to_string(), DEFAULT_DOWN)?,
                    ),
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
    db: Arc<Store>,
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
