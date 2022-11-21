use std::{collections::HashSet, env, path::PathBuf, sync::Arc};

use anyhow::{Error, Result};
use chrono::{TimeZone, Utc};
use plotters::prelude::*;
use teloxide::{
    adaptors::DefaultParseMode,
    payloads::SendPhotoSetters,
    requests::{Requester, ResponseResult},
    types::{InputFile, Message},
    utils::command::BotCommands,
    Bot,
};
use tokio::fs;

use super::{mention_chat, mention_id, mention_user};
use crate::db::{Measure, Store};

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum GroupCommand {
    #[command(description = "display leaderboard.")]
    Leaderboard,
    #[command(description = "display graph.")]
    Chart,
}

const MARGIN: i32 = 10;
const LABEL_AREA: i32 = 40;

fn graph(path: &PathBuf, data: Vec<Measure>) -> Result<()> {
    let root = BitMapBackend::new(path, (640, 480)).into_drawing_area();

    root.fill(&WHITE)?;
    let surface = root.margin(MARGIN + LABEL_AREA, MARGIN, MARGIN, MARGIN + LABEL_AREA);

    let max_karma = data.iter().map(|m| m.karma).max().unwrap_or(0) + 1;
    let min_karma = data.iter().map(|m| m.karma).min().unwrap_or(0) - 1;

    let mut chart = ChartBuilder::on(&surface)
        .set_label_area_size(LabelAreaPosition::Left, LABEL_AREA)
        .set_label_area_size(LabelAreaPosition::Bottom, LABEL_AREA)
        .build_cartesian_2d(0..(data.len() - 1), min_karma..max_karma)?;

    chart
        .configure_mesh()
        .x_label_formatter(&|x| {
            format!(
                "{}",
                Utc.timestamp(data[*x].timestamp, 0).format("%d/%m %H:%M")
            )
        })
        .y_desc("karma")
        .x_desc("time")
        .draw()?;

    chart.draw_series(LineSeries::new(
        (0..).zip(data.iter()).map(|(i, m)| (i, m.karma)),
        BLUE,
    ))?;

    Ok(())
}

async fn handler(
    bot: DefaultParseMode<Bot>,
    db: Arc<Store>,
    msg: Message,
    cmd: GroupCommand,
) -> Result<()> {
    match cmd {
        GroupCommand::Leaderboard => {
            let members = db.members.get_or(msg.chat.id.to_string(), HashSet::new())?;

            if members.is_empty() {
                let text = "<i>There are no members with karma in this group.</i>";
                bot.send_message(msg.chat.id, text).await?;
                return Ok(());
            }

            let mut leaderboard = members
                .iter()
                .map(|id| {
                    let karma = db.karma.get_or(id.to_string(), 0)?;
                    Ok((id, karma))
                })
                .collect::<Result<Vec<_>, Error>>()?;

            leaderboard.sort_by(|(_, a), (_, b)| b.cmp(a));

            let mut text = String::new();
            for (i, (id, karma)) in leaderboard.iter().enumerate() {
                if let Ok(chat) = bot.get_chat(**id).await {
                    let mention = mention_chat(&chat);
                    text.push_str(&format!("{}. {} : {}", i + 1, mention, karma));
                    text.push('\n');
                } else {
                    let mention = mention_id(id);
                    text.push_str(&format!("{}. {} : {}", i + 1, mention, karma));
                    text.push('\n');
                }
            }

            let last_message_key = format!("{}-leaderboard", msg.chat.id);
            if let Some(last_message) = db.last_message.get(&last_message_key)? {
                bot.delete_message(msg.chat.id, last_message).await.ok();
            }

            let message = bot.send_message(msg.chat.id, text).await?;
            db.last_message.insert(&last_message_key, message.id)?;
        }
        GroupCommand::Chart => {
            if let Some(mut user) = msg.from() {
                // if command is a reply to a message by another user, use that user
                if let Some(reply) = msg.reply_to_message() {
                    if let Some(other) = reply.from() {
                        user = other;
                    }
                }

                let data = db.graph.get_or(user.id.to_string(), vec![])?;

                if data.len() < 2 {
                    let text = "<i>There is no data to display.</i>";
                    bot.send_message(msg.chat.id, text).await?;
                    return Ok(());
                }

                let path = env::temp_dir().join(format!("{}.png", user.id));

                graph(&path, data)?;

                let file = InputFile::file(&path);
                let caption = format!("Karma chart for {}", mention_user(user));

                let last_message_key = format!("{}-chart", msg.chat.id);
                if let Some(last_message) = db.last_message.get(&last_message_key)? {
                    bot.delete_message(msg.chat.id, last_message).await.ok();
                }

                let message = bot.send_photo(msg.chat.id, file).caption(caption).await?;
                db.last_message.insert(&last_message_key, message.id)?;

                fs::remove_file(&path).await?;
            }
        }
    };

    Ok(())
}

pub async fn command_handler(
    bot: DefaultParseMode<Bot>,
    db: Arc<Store>,
    msg: Message,
    cmd: GroupCommand,
) -> ResponseResult<()> {
    match handler(bot, db, msg, cmd).await {
        Ok(_) => Ok(()),
        Err(err) => match err.downcast::<teloxide::RequestError>() {
            Ok(err) => Err(err),
            Err(err) => {
                log::error!("Generic error: {}", err);
                Ok(())
            }
        },
    }
}
