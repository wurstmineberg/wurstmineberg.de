import contextlib
import json

DEFAULT_DB_CONFIG = {
    "connectionstring": "postgresql://localhost/wurstmineberg"
}

def get_db_config(config_filename='/opt/wurstmineberg/config/database.json'):
    config = DEFAULT_DB_CONFIG.copy()
    with contextlib.suppress(FileNotFoundError):
        with open(config_filename) as cfg_file:
            config.update(json.load(cfg_file))

    return config
