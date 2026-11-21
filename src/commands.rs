use pumpkin_plugin_api::command::{
    Command, CommandError, CommandNode, CommandSender, ConsumedArgs,
};
use pumpkin_plugin_api::commands::CommandHandler;
use pumpkin_plugin_api::common::RgbColor;
use pumpkin_plugin_api::permission::{Permission, PermissionDefault};
use pumpkin_plugin_api::text::TextComponent;
use pumpkin_plugin_api::{Context, Result, Server};

use super::{AUTHORS, VERSION};

const COMMAND_PERMISSION: &str = "rummo:command.rummo";
const BANNER: &str = r"
██████╗ ██╗   ██╗███╗   ███╗███╗   ███╗ ██████╗ 
██╔══██╗██║   ██║████╗ ████║████╗ ████║██╔═══██╗
██████╔╝██║   ██║██╔████╔██║██╔████╔██║██║   ██║
██╔══██╗██║   ██║██║╚██╔╝██║██║╚██╔╝██║██║   ██║
██║  ██║╚██████╔╝██║ ╚═╝ ██║██║ ╚═╝ ██║╚██████╔╝
╚═╝  ╚═╝ ╚═════╝ ╚═╝     ╚═╝╚═╝     ╚═╝ ╚═════╝ 
░░░░░░   ░░░░░   ░░      ░░ ░░      ░░  ░░░░░";

pub(super) fn register(context: &Context) -> Result<()> {
    let description = "Description";
    context.register_permission(&Permission {
        node: COMMAND_PERMISSION.into(),
        description: description.into(),
        default: PermissionDefault::Allow,
        children: Vec::new(),
    })?;

    let command = Command::new(&["rummo".into()], description)
        .execute(ShowInfo)
        .then(CommandNode::literal("help").execute(ShowHelp));
    context.register_command(command, COMMAND_PERMISSION);
    Ok(())
}

struct ShowInfo;

impl CommandHandler for ShowInfo {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let banner = TextComponent::text(BANNER).color_rgb(RgbColor {
            r: 255,
            g: 165,
            b: 0,
        });
        let message = TextComponent::text("").add_child(banner).add_text(&format!(
            "\n作者 {}  |  版本 {}",
            AUTHORS.replace(':', ", "),
            VERSION
        ));
        sender.send_message(message);
        Ok(0)
    }
}

struct ShowHelp;

impl CommandHandler for ShowHelp {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        sender.send_message(TextComponent::text("/rummo help — 列出可用指令"));
        Ok(0)
    }
}
