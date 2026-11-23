use pumpkin_plugin_api::command::{
    Arg, ArgumentType, Command, CommandError, CommandNode, CommandSender, ConsumedArgs, StringType,
};
use pumpkin_plugin_api::command_wit::{Number, PermissionLevel as CommandPermissionLevel};
use pumpkin_plugin_api::commands::CommandHandler;
use pumpkin_plugin_api::common::RgbColor;
use pumpkin_plugin_api::permission::{Permission, PermissionDefault, PermissionLevel};
use pumpkin_plugin_api::text::TextComponent;
use pumpkin_plugin_api::{Context, Player, Result, Server};

use crate::skills::skill_manager::{SkillInfo, SkillManager, SkillType};

use super::{AUTHORS, VERSION};

const RUMMO_PERMISSION: &str = "rummo:command.rummo";
const SET_EXP_PERMISSION: &str = "rummo:commands.setexp";
const SET_LEVEL_PERMISSION: &str = "rummo:commands.setlevel";
const ADD_EXP_PERMISSION: &str = "rummo:commands.addexp";
const ADD_LEVEL_PERMISSION: &str = "rummo:commands.addlevel";
const MINING_PERMISSION: &str = "rummo:commands.mining";

const ADMIN_COMMANDS: &[EditSkill] = &[
    EditSkill {
        name: "setexp",
        permission: SET_EXP_PERMISSION,
        description: "设置玩家的技能经验",
        argument: "exp",
        minimum: 0,
        update: SkillManager::set_exp,
    },
    EditSkill {
        name: "setlevel",
        permission: SET_LEVEL_PERMISSION,
        description: "设置玩家的技能等级",
        argument: "level",
        minimum: 1,
        update: SkillManager::set_level,
    },
    EditSkill {
        name: "addexp",
        permission: ADD_EXP_PERMISSION,
        description: "增加玩家的技能经验",
        argument: "exp",
        minimum: 0,
        update: SkillManager::add_exp,
    },
    EditSkill {
        name: "addlevel",
        permission: ADD_LEVEL_PERMISSION,
        description: "增加玩家的技能等级",
        argument: "level",
        minimum: 0,
        update: SkillManager::add_level,
    },
];
const SKILL_COMMANDS: &[(SkillType, &str)] = &[(SkillType::Mining, MINING_PERMISSION)];

const BANNER: &str = r"
██████╗ ██╗   ██╗███╗   ███╗███╗   ███╗ ██████╗ 
██╔══██╗██║   ██║████╗ ████║████╗ ████║██╔═══██╗
██████╔╝██║   ██║██╔████╔██║██╔████╔██║██║   ██║
██╔══██╗██║   ██║██║╚██╔╝██║██║╚██╔╝██║██║   ██║
██║  ██║╚██████╔╝██║ ╚═╝ ██║██║ ╚═╝ ██║╚██████╔╝
╚═╝  ╚═╝ ╚═════╝ ╚═╝     ╚═╝╚═╝     ╚═╝ ╚═════╝ 
░░░░░░   ░░░░░   ░░      ░░ ░░      ░░  ░░░░░";

pub(super) fn register(context: &Context) -> Result<()> {
    let description = "查看 Rummo 信息和指令帮助";
    let mut rummo = Command::new(&["rummo".into()], description)
        .execute(ShowInfo)
        .then(CommandNode::literal("help").execute(ShowHelp));

    for &command in ADMIN_COMMANDS {
        rummo = register_command(
            context,
            command.permission,
            command.description,
            PermissionDefault::Op(PermissionLevel::Two),
            |_| rummo.then(command.node()),
        )?;
    }
    register_command(
        context,
        RUMMO_PERMISSION,
        description,
        PermissionDefault::Allow,
        |node| {
            context.register_command(rummo, node);
        },
    )?;

    for &(skill, permission) in SKILL_COMMANDS {
        let description = format!("查看自己的{}技能详情", skill.display_name());
        register_command(
            context,
            permission,
            &description,
            PermissionDefault::Allow,
            |node| {
                let command =
                    Command::new(&[skill.id().into()], &description).execute(ShowSkill(skill));
                context.register_command(command, node);
            },
        )?;
    }
    Ok(())
}

fn register_command<T>(
    context: &Context,
    node: &str,
    description: &str,
    default: PermissionDefault,
    register: impl FnOnce(&str) -> T,
) -> Result<T> {
    context.register_permission(&Permission {
        node: node.into(),
        description: description.into(),
        default,
        children: Vec::new(),
    })?;
    Ok(register(node))
}

#[derive(Clone, Copy)]
struct EditSkill {
    name: &'static str,
    permission: &'static str,
    description: &'static str,
    argument: &'static str,
    minimum: u32,
    update: fn(&Player, SkillType, u32) -> Result<SkillInfo>,
}

impl EditSkill {
    fn node(self) -> CommandNode {
        let value = CommandNode::argument(
            self.argument,
            &ArgumentType::Long((Some(i64::from(self.minimum)), Some(i64::from(u32::MAX)))),
        )
        .execute(self);
        let skill = CommandNode::argument("skill", &ArgumentType::String(StringType::SingleWord))
            .then(value);
        let player = CommandNode::argument("Player", &ArgumentType::Players).then(skill);
        CommandNode::literal(self.name).then(player)
    }

    fn can_execute(&self, sender: &CommandSender, server: &Server) -> bool {
        // Pumpkin attaches permission nodes to the root command only.
        sender.has_permission_level(CommandPermissionLevel::Two)
            && sender.has_permission(server, self.permission)
    }
}

impl CommandHandler for EditSkill {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        if !self.can_execute(&sender, &server) {
            return Err(command_failure("此指令需要至少 2 级 OP 和对应的权限。"));
        }

        let player = match args.get_value("Player") {
            Arg::Players(mut players) if players.len() == 1 => players.pop(),
            _ => None,
        }
        .ok_or_else(|| command_failure("请选择一名在线玩家。"))?;
        let Arg::Simple(skill_id) = args.get_value("skill") else {
            return Err(command_failure("请输入技能 ID。"));
        };
        let skill = SkillType::from_id(&skill_id)
            .ok_or_else(|| command_failure(&format!("未知技能「{skill_id}」。")))?;
        let value = match args.get_value(self.argument) {
            Arg::Num(Ok(Number::Int64(value)))
                if (i64::from(self.minimum)..=i64::from(u32::MAX)).contains(&value) =>
            {
                value as u32
            }
            _ => {
                return Err(command_failure(&format!(
                    "请输入 {} 到 {} 之间的整数。",
                    self.minimum,
                    u32::MAX,
                )));
            }
        };

        let name = player.get_name();
        let info = (self.update)(&player, skill, value).map_err(skill_error)?;
        sender.send_message(TextComponent::text(&format!(
            "已更新 {name} 的技能：{}",
            skill_details(skill, info),
        )));
        Ok(0)
    }
}

struct ShowSkill(SkillType);

impl CommandHandler for ShowSkill {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let player = sender
            .as_player()
            .ok_or_else(|| command_failure("此指令只能由玩家执行。"))?;
        let info = SkillManager::get_skill(&player, self.0).map_err(skill_error)?;
        sender.send_message(TextComponent::text(&skill_details(self.0, info)));
        Ok(0)
    }
}

fn skill_details(skill: SkillType, info: SkillInfo) -> String {
    let name = skill.display_name();
    let SkillInfo {
        level,
        experience,
        required_experience,
        ..
    } = info;
    match required_experience {
        Some(required) => format!("{name} Lv.{level} — 本级经验 {experience} / {required} XP"),
        None => format!("{name} Lv.{level} — 已满级，当前经验 {experience} XP"),
    }
}

fn command_failure(message: &str) -> CommandError {
    CommandError::CommandFailed(TextComponent::text(message))
}

fn skill_error(error: String) -> CommandError {
    tracing::error!(%error, "Skill command failed");
    command_failure("技能操作失败，请查看服务器日志。")
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
        server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        sender.send_message(TextComponent::text("/rummo help — 列出可用指令"));
        for &(skill, permission) in SKILL_COMMANDS {
            if sender.is_player() && sender.has_permission(&server, permission) {
                sender.send_message(TextComponent::text(&format!(
                    "/{} — 查看自己的{}技能详情",
                    skill.id(),
                    skill.display_name(),
                )));
            }
        }
        for command in ADMIN_COMMANDS {
            if command.can_execute(&sender, &server) {
                sender.send_message(TextComponent::text(&format!(
                    "/rummo {} <Player> <skill> <{}> — {}",
                    command.name, command.argument, command.description,
                )));
            }
        }
        Ok(0)
    }
}
