import re

import wurstmineberg_web.models

DISCORD_MENTION_REGEX = f'<@!?({wurstmineberg_web.models.WMBID_REGEX}|[0-9]+)>'

def mentions_to_tags(text):
    while True:
        match = re.search(DISCORD_MENTION_REGEX, text)
        if not match:
            return text
        person = wurstmineberg_web.models.Person.from_snowflake_or_wmbid(match.group(1))
        if person.discorddata is None:
            tag = f'@{person.wmbid}#'
        else:
            tag = f'@{person.discorddata["username"]}#{person.discorddata["discriminator"]:04}'
        text = f'{text[:match.start()]}{tag}{text[match.end():]}'

def tags_to_mentions(text):
    while True:
        match = re.search(DISCORD_TAG_REGEX, text)
        if not match:
            return text
        person = wurstmineberg_web.models.Person.from_tag(match.group(1), None if match.group(2) == '' else int(match.group(2)))
        text = f'{text[:match.start()]}<@{person.snowflake_or_wmbid}>{text[match.end():]}'
