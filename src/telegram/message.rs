use std::{collections::HashSet, fmt::Display, str::FromStr, sync::Arc};

use bincode::{deserialize, serialize};
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use sled::Db;
use teloxide::{
    adaptors::DefaultParseMode,
    requests::{Requester, ResponseResult},
    types::{Message, MessageId, UserId},
    Bot,
};

use crate::{db, error::Error};

#[derive(Debug)]
pub enum Karma {
    Up,
    Down,
}

impl Display for Karma {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Karma::Up => write!(f, "+"),
            Karma::Down => write!(f, "-"),
        }
    }
}

impl FromStr for Karma {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with('+') {
            Ok(Karma::Up)
        } else if s.starts_with('-') {
            Ok(Karma::Down)
        } else {
            Err(())
        }
    }
}

async fn handler(bot: DefaultParseMode<Bot>, db: Arc<Db>, msg: Message) -> Result<(), Error> {
    if let Some(Ok(modifier)) = msg.text().map(|text| Karma::from_str(text)) {
        if let Some(reply) = msg.reply_to_message() {
            if let (Some(giver), Some(receiver)) = (msg.from(), reply.from()) {
                if !giver.is_bot && !receiver.is_bot && giver.id != receiver.id {
                    let db_up = db.open_tree(db::TREE_UP)?;
                    let db_down = db.open_tree(db::TREE_DOWN)?;
                    let db_karma = db.open_tree(db::TREE_KARMA)?;
                    let db_last = db.open_tree(db::TREE_LAST)?;
                    let db_last_message = db.open_tree(db::TREE_LAST_MESSAGE)?;
                    let db_members = db.open_tree(db::TREE_MEMBERS)?;

                    let current_last = db_last
                        .get(giver.id.to_string())?
                        .map_or(Ok(db::DEFAULT_LAST), |bytes| deserialize(&bytes))?;

                    let now = Utc::now();
                    let then = DateTime::<Utc>::from_utc(
                        NaiveDateTime::from_timestamp(current_last, 0),
                        Utc,
                    );

                    let midnight = (then + Duration::days(1)).date().and_hms(0, 0, 0);
                    let expired = now.gt(&midnight);

                    if expired {
                        db_up.remove(giver.id.to_string())?;
                        db_down.remove(giver.id.to_string())?;
                    }

                    let (db_available, default_available) = match modifier {
                        Karma::Up => (&db_up, db::DEFAULT_UP),
                        Karma::Down => (&db_down, db::DEFAULT_DOWN),
                    };

                    let available_current = db_available
                        .get(giver.id.to_string())?
                        .map_or(Ok(default_available), |bytes| deserialize(&bytes))?;

                    if available_current < 1 {
                        let text = format!("<i>no more {} points available today</i>", modifier);
                        bot.send_message(msg.chat.id, text).await?;
                        return Ok(());
                    }

                    let available = available_current - 1;

                    let karma_current = db_karma
                        .get(receiver.id.to_string())?
                        .map_or(Ok(db::DEFAULT_KARMA), |bytes| deserialize(&bytes))?;

                    let karma = match modifier {
                        Karma::Up => karma_current + 1,
                        Karma::Down => karma_current - 1,
                    };

                    let timestamp = now.naive_utc().timestamp();
                    db_last.insert(giver.id.to_string(), serialize(&timestamp)?)?;
                    db_available.insert(giver.id.to_string(), serialize(&available)?)?;
                    db_karma.insert(receiver.id.to_string(), serialize(&karma)?)?;

                    let mut members = db_members
                        .get(msg.chat.id.to_string())?
                        .map_or(Ok(HashSet::<UserId>::new()), |bytes| deserialize(&bytes))?;

                    members.insert(giver.id);
                    members.insert(receiver.id);

                    db_members.insert(msg.chat.id.to_string(), serialize(&members)?)?;

                    let last_message_key = format!("{}-{}", msg.chat.id, receiver.id);
                    let last_message: Option<MessageId> =
                        db_last_message
                            .get(&last_message_key)?
                            .map_or(Ok(None), |bytes| deserialize(&bytes).map(|id| Some(id)))?;

                    if let Some(last_message) = last_message {
                        bot.delete_message(msg.chat.id, last_message).await.ok();
                    }

                    let text = format!(
                        "reputation of {} ({})",
                        receiver.mention().unwrap_or(receiver.full_name()),
                        karma
                    );

                    let update_message = bot.send_message(msg.chat.id, text).await?;
                    db_last_message.insert(&last_message_key, serialize(&update_message.id)?)?;
                }
            }
        }
    }

    Ok(())
}

pub async fn message_handler(
    bot: DefaultParseMode<Bot>,
    db: Arc<Db>,
    msg: Message,
) -> ResponseResult<()> {
    match handler(bot, db, msg).await {
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
