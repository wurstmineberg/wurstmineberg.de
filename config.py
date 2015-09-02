import contextlib

DEFAULT_DB_CONFIG = {
    "connectionstring": "host=localhost dbname=wurstmineberg",
}

def get_db_config(config_filename='/opt/wurstmineberg/config/database.json'):
    config = DEFAULT_DB_CONFIG.copy()
    with contextlib.suppress(FileNotFoundError):
        with open(config_filename) as cfg_file:
            config.update(json.load(cfg_file))

    return config
