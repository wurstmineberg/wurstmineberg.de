import pathlib
import subprocess

EXEC_PATH = pathlib.Path('/opt/git/github.com/wurstmineberg/wurstminebot-discord/master/target/release/wurstminebot')

class CommandError(RuntimeError):
    pass

def cmd(cmd, *args, expected_response=object()):
    response = subprocess.run([str(EXEC_PATH), cmd, *args], check=True, stdout=subprocess.PIPE).stdout.decode('utf-8').strip()
    if response != expected_response:
        raise CommandError('{} command failed with response: {}'.format(cmd, response))

def escape(text):
    text = str(text)
    #FROM https://docs.rs/serenity/0.7.4/src/serenity/utils/message_builder.rs.html#556-568
    # Remove invite links and popular scam websites, mostly to prevent the
    # current user from triggering various ad detectors and prevent embeds.
    text = text.replace('discord.gg', 'discord\u2024gg')
    text = text.replace('discord.me', 'discord\u2024me')
    text = text.replace('discordlist.net', 'discordlist\u2024net')
    text = text.replace('discordservers.com', 'discordservers\u2024com')
    text = text.replace('discordapp.com/invite', 'discordapp\u2024com/invite')
    text = text.replace('discord.com/invite', 'discord\u2024com/invite')
    # Remove right-to-left override and other similar annoying symbols
    text = text.replace('\u202e', ' ') # RTL Override
    text = text.replace('\u200f', ' ') # RTL Mark
    text = text.replace('\u202b', ' ') # RTL Embedding
    text = text.replace('\u200b', ' ') # Zero-width space
    text = text.replace('\u200d', ' ') # Zero-width joiner
    text = text.replace('\u200c', ' ') # Zero-width non-joiner
    # Remove everyone and here mentions. Has to be put after ZWS replacement
    # because it utilises it itself.
    text = text.replace('@everyone', '@\u200beveryone')
    text = text.replace('@here', '@\u200bhere')
    return text.replace('*', '\\*').replace('`', '\\`').replace('_', '\\_')

# one function for every IPC command implemented in listen_ipc

def channel_msg(channel, msg):
    cmd('channel-msg', str(channel), msg, expected_response='message sent')

def quit():
    cmd('quit', expected_response='shutdown complete')

def set_display_name(user, display_name):
    cmd('set-display-name', str(user.snowflake), display_name, expected_response='display name set')
