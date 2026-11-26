use pumpkin_plugin_api::command::{
    Arg, ArgumentType, Command, CommandError, CommandNode, CommandSender, ConsumedArgs, StringType,
};
use pumpkin_plugin_api::command_wit::{Number, PermissionLevel as CommandPermissionLevel};
use pumpkin_plugin_api::commands::CommandHandler;
use pumpkin_plugin_api::common::RgbColor;
use pumpkin_plugin_api::permission::{Permission, PermissionDefault, PermissionLevel};
use pumpkin_plugin_api::text::TextComponent;
use pumpkin_plugin_api::{Context, Player, Result, Server};

use crate::i18n::{self, Messages};
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
        description_key: "command.description.setexp",
        argument: "exp",
        minimum: 0,
        update: SkillManager::set_exp,
    },
    EditSkill {
        name: "setlevel",
        permission: SET_LEVEL_PERMISSION,
        description_key: "command.description.setlevel",
        argument: "level",
        minimum: 1,
        update: SkillManager::set_level,
    },
    EditSkill {
        name: "addexp",
        permission: ADD_EXP_PERMISSION,
        description_key: "command.description.addexp",
        argument: "exp",
        minimum: 0,
        update: SkillManager::add_exp,
    },
    EditSkill {
        name: "addlevel",
        permission: ADD_LEVEL_PERMISSION,
        description_key: "command.description.addlevel",
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
    let messages = i18n::server_default();
    let description = messages.text("command.description.rummo");
    let mut rummo = Command::new(&["rummo".into()], description)
        .execute(ShowInfo)
        .then(CommandNode::literal("help").execute(ShowHelp));

    for &command in ADMIN_COMMANDS {
        rummo = register_command(
            context,
            command.permission,
            messages.text(command.description_key),
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
        let description = skill_description(&messages, skill);
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
    description_key: &'static str,
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
        let messages = i18n::for_sender(&sender);
        if !self.can_execute(&sender, &server) {
            return Err(command_failure(messages.text("command.error.permission")));
        }

        let player = match args.get_value("Player") {
            Arg::Players(mut players) if players.len() == 1 => players.pop(),
            _ => None,
        }
        .ok_or_else(|| command_failure(messages.text("command.error.player")))?;
        let Arg::Simple(skill_id) = args.get_value("skill") else {
            return Err(command_failure(messages.text("command.error.skill_id")));
        };
        let skill = SkillType::from_id(&skill_id).ok_or_else(|| {
            command_failure(
                &messages.format("command.error.unknown_skill", &[("skill", &skill_id)]),
            )
        })?;
        let value = match args.get_value(self.argument) {
            Arg::Num(Ok(Number::Int64(value)))
                if (i64::from(self.minimum)..=i64::from(u32::MAX)).contains(&value) =>
            {
                value as u32
            }
            _ => {
                return Err(command_failure(&messages.format(
                    "command.error.range",
                    &[("min", &self.minimum), ("max", &u32::MAX)],
                )));
            }
        };

        let name = player.get_name();
        let info =
            (self.update)(&player, skill, value).map_err(|error| skill_error(&messages, error))?;
        sender.send_message(TextComponent::text(&messages.format(
            "command.updated",
            &[
                ("player", &name),
                ("details", &skill_details(&messages, skill, info)),
            ],
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
        let messages = i18n::for_sender(&sender);
        let player = sender
            .as_player()
            .ok_or_else(|| command_failure(messages.text("command.error.player_only")))?;
        let info = SkillManager::get_skill(&player, self.0)
            .map_err(|error| skill_error(&messages, error))?;
        sender.send_message(TextComponent::text(&skill_details(&messages, self.0, info)));
        Ok(0)
    }
}

fn skill_description(messages: &Messages, skill: SkillType) -> String {
    messages.format(
        "command.description.skill",
        &[("skill", &messages.text(skill.name_key()))],
    )
}

fn skill_details(messages: &Messages, skill: SkillType, info: SkillInfo) -> String {
    let name = messages.text(skill.name_key());
    let SkillInfo {
        level,
        experience,
        required_experience,
        ..
    } = info;
    match required_experience {
        Some(required) => messages.format(
            "skill.details",
            &[
                ("skill", &name),
                ("level", &level),
                ("experience", &experience),
                ("required", &required),
            ],
        ),
        None => messages.format(
            "skill.details_max",
            &[
                ("skill", &name),
                ("level", &level),
                ("experience", &experience),
            ],
        ),
    }
}

fn command_failure(message: &str) -> CommandError {
    CommandError::CommandFailed(TextComponent::text(message))
}

fn skill_error(messages: &Messages, error: String) -> CommandError {
    tracing::error!(%error, "Skill command failed");
    command_failure(messages.text("command.error.skill_failed"))
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
        let info = i18n::for_sender(&sender).format(
            "command.info",
            &[
                ("authors", &AUTHORS.replace(':', ", ")),
                ("version", &VERSION),
            ],
        );
        let message = TextComponent::text("")
            .add_child(banner)
            .add_text(&format!("\n{info}"));
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
        let messages = i18n::for_sender(&sender);
        sender.send_message(TextComponent::text(&format!(
            "/rummo help — {}",
            messages.text("command.help"),
        )));
        for &(skill, permission) in SKILL_COMMANDS {
            if sender.is_player() && sender.has_permission(&server, permission) {
                sender.send_message(TextComponent::text(&format!(
                    "/{} — {}",
                    skill.id(),
                    skill_description(&messages, skill),
                )));
            }
        }
        for command in ADMIN_COMMANDS {
            if command.can_execute(&sender, &server) {
                sender.send_message(TextComponent::text(&format!(
                    "/rummo {} <Player> <skill> <{}> — {}",
                    command.name,
                    command.argument,
                    messages.text(command.description_key),
                )));
            }
        }
        Ok(0)
    }
}
