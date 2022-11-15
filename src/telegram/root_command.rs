use std::sync::Arc;

use teloxide::{
    adaptors::DefaultParseMode,
    requests::{Requester, ResponseResult},
    types::{Message, UserId},
    utils::command::BotCommands,
    Bot,
};

use crate::{db::Store, error::Error};

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum RootCommand {
    #[command(description = "reset expiration [admin].")]
    ResetAll,
    #[command(description = "reset user expiration [admin].")]
    Reset(u64),
    #[command(description = "identify user [admin].")]
    Info,
}

async fn handler(
    bot: DefaultParseMode<Bot>,
    db: Arc<Store>,
    root: UserId,
    msg: Message,
    cmd: RootCommand,
) -> Result<(), Error> {
    match cmd {
        RootCommand::Reset(user) => {
            db.last.remove(user.to_string())?;
            bot.send_message(root, "Reset complete.").await?;
        }
        RootCommand::ResetAll => {
            db.last.clear()?;
            bot.send_message(root, "Reset complete.").await?;
        }
        RootCommand::Info => {
            if let Some(reply) = msg.reply_to_message() {
                if let Some(user) = reply.from() {
                    let text = format!(
                        "User info {}: \n\
                                - ID: {}\n\
                                - Username: {}",
                        user.full_name(),
                        user.id,
                        user.username.clone().unwrap_or("N/A".to_string())
                    );
                    bot.send_message(root, text).await?;
                    bot.delete_message(msg.chat.id, msg.id).await.ok();
                }
            }
        }
    };

    Ok(())
}

pub async fn command_handler(
    bot: DefaultParseMode<Bot>,
    db: Arc<Store>,
    root: UserId,
    msg: Message,
    cmd: RootCommand,
) -> ResponseResult<()> {
    match handler(bot, db, root, msg, cmd).await {
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
