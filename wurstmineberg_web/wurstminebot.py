import pathlib
import subprocess

EXEC_PATH = pathlib.Path('/opt/git/github.com/wurstmineberg/wurstminebot-discord/master/target/release/wurstminebot')

class CommandError(RuntimeError):
    pass

def cmd(cmd, *args, expected_response=object()):
    response = subprocess.run([str(EXEC_PATH), cmd, *args], check=True, stdout=subprocess.PIPE).stdout.decode('utf-8').strip()
    if response != expected_response:
        raise CommandError('{} command failed with response: {}'.format(cmd, response))

# one function for every IPC command implemented in listen_ipc

def quit():
    cmd('quit', expected_response='shutdown complete')

def set_display_name(user, display_name):
    cmd('set-display-name', str(user.snowflake), display_name, expected_response='display name set')
