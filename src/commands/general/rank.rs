use serenity::{
    client::Context,
    model::{
        channel::Message,
        id::{
            UserId,
            GuildId
        },
        user::User
    },
    framework::standard::{
        CommandResult,
        macros::{
            command
        },
        Args
    },
    utils::MessageBuilder
};
use crate::{Database, db};
use log::{error};

async fn rank_embed(ctx: &Context, msg: &Message, server_id: &GuildId, user: &User) {
    let db = db!(ctx);

    let (xp, level) = db.get_xp(server_id.clone(), user.id).await.unwrap();
    let next_level_xp = db.calculate_level(level).await.unwrap();

    let current_role = db.get_highest_role(server_id.clone(), level).await.unwrap();
    let mut current_role_str: String = String::from("No role");
    if let Some(current_role_id) = current_role {
        current_role_str = format!("Current role: <@&{}>", current_role_id);
    }

    let mut pfp_url = user.default_avatar_url();
    if let Some(pfp_custom) = user.avatar_url() {
        pfp_url = pfp_custom;
    }

    let mut rank_str = String::from("(Unranked)");
    if let Some(rank) = db.rank_within_members(server_id.clone(), user.id).await.unwrap() {
        rank_str = format!("#{}", rank);
    }

    if let Err(ex) = msg.channel_id.send_message(&ctx.http, |m| {m.embed(|e| {
        e
            .title(
                MessageBuilder::new()
                    .push_safe(user.name.as_str())
                    .push("#")
                    .push(user.discriminator)
                    .push("'s Ranking")
                    .build()
            )
            .description(current_role_str)
            .field("XP", format!("{}/{}", xp, next_level_xp), true)
            .field("Rank", rank_str, true)
            .thumbnail(pfp_url)
    })}).await {
        error!("Failed to send embed: {}", ex);
    }
}

#[command]
#[description = "Get your current rank."]
#[only_in(guilds)]
pub async fn rank(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let other = args.single::<UserId>();
    if let Some(server_id) = msg.guild_id {
        if let Ok(other_id) = other {
            if let Ok(other_user) = other_id.to_user(&ctx.http).await {
                rank_embed(ctx, msg, &server_id, &other_user).await;
            } else {
                msg.channel_id.say(&ctx.http, "Could not find user...").await?;
            }
        } else {
            rank_embed(ctx, msg, &server_id, &msg.author).await;
        }
    } else {
        msg.reply(&ctx.http, "This command can only be run in a server.").await?;
    }

    Ok(())
}

#[command]
#[description = "Disable/enable experience from being collected in the current channel."]
#[only_in(guilds)]
#[required_permissions("ADMINISTRATOR")]
pub async fn disablexp(ctx: &Context, msg: &Message) -> CommandResult {
    let db = db!(ctx);
    if let Some(server_id) = msg.guild_id {
        let mut content: String;
        match db.toggle_channel_xp(server_id, msg.channel_id).await {
            Ok(toggle) => {
                if toggle {
                    content = format!("Disabled");
                } else {
                    content = format!("Enabled");
                }
                content += &*format!(" collecting experience in <#{}>.", msg.channel_id.as_u64());
            },
            Err(ex) => {
                content = format!("Failed to toggle channel xp status.");
                error!("Failed to toggle channel xp status: {}", ex);
            }
        }

        msg.channel_id.send_message(&ctx.http, |m| {m.content(content)}).await?;
    } else {
        msg.reply(&ctx.http, "This command can only be run in a server.").await?;
    }

    Ok(())
}

#[command]
#[description = "Get the current rankings in the server."]
#[only_in(guilds)]
pub async fn levels(ctx: &Context, msg: &Message) -> CommandResult {
    let db = db!(ctx);
    if let Some(server_id) = msg.guild_id {
        let content: String;
        match db.top_members(server_id).await {
            Ok(users) => {
               content = users.into_iter()
                   .map(|u| {
                       let (id, level, xp) = u;
                       format!("<@{}> - Level {}, {} xp", id, level, xp)
                   })
                   .reduce(|a, b| {format!("{}\n{}", a, b)})
                   .unwrap_or(String::from("Nobody is ranked in this server."))
            },
            Err(ex) => {
                content = format!("Failed to get rankings.");
                error!("Failed to get rankings: {}", ex);
            }
        }

        msg.channel_id.send_message(&ctx.http, |m| {
            m.embed(|e|
                e.title("Top Users")
                    .description(content)
            )}).await?;
    } else {
        msg.reply(&ctx.http, "This command can only be run in a server.").await?;
    }

    Ok(())
}