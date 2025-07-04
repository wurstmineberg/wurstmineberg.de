import re
import xml.etree.ElementTree

import markdown # PyPI: Markdown
import markdown.inlinepatterns # PyPI: Markdown

import wurstminebot # https://github.com/wurstmineberg/wurstminebot-discord

import wurstmineberg_web.models

CHANNEL_ID = 681458815543148547
DISCORD_OR_WMBID_MENTION_REGEX = re.compile(f'<@!?({wurstmineberg_web.models.WMBID_REGEX.pattern}|[0-9]+)>')
DISCORD_OR_WMBID_TAG_REGEX = re.compile('@([^@#:\n]{2,32})#((?:[0-9]{4})?)') # see https://discord.com/developers/docs/resources/user
WMBID_MENTION_REGEX = f'<@!?({wurstmineberg_web.models.WMBID_REGEX.pattern})>'

class WmbidMentionPattern(markdown.inlinepatterns.LinkInlineProcessor):
    def handleMatch(self, m, data):
        user = wurstmineberg_web.models.Person.from_wmbid(m.group(1))
        el = xml.etree.ElementTree.Element('a')
        el.text = f'@{user.name}'
        el.set('href', user.profile_url)
        return el, m.start(0), m.end(0)

class WmbidMentionExtension(markdown.Extension):
    def extendMarkdown(self, md, md_globals=None):
        md.inlinePatterns.register(WmbidMentionPattern(WMBID_MENTION_REGEX, md), 'wmbid-mention', 169)

def mentions_to_tags(text):
    while True:
        match = DISCORD_OR_WMBID_MENTION_REGEX.search(text)
        if not match:
            return text
        person = wurstmineberg_web.models.Person.from_snowflake_or_wmbid(match.group(1))
        if person.discorddata is None:
            tag = f'@{person.wmbid}#'
        elif person.discorddata['discriminator'] is None:
            tag = f'@{person.discorddata["username"]}#'
        else:
            tag = f'@{person.discorddata["username"]}#{person.discorddata["discriminator"]:04}'
        text = f'{text[:match.start()]}{tag}{text[match.end():]}'

def save_hook(namespace, title, text, author, summary, created):
    raise NotImplementedError('This code should be unreachable since the endpoint calling it has been ported to Rust')

def tags_to_mentions(text):
    while True:
        match = DISCORD_OR_WMBID_TAG_REGEX.search(text)
        if not match:
            return text
        person = wurstmineberg_web.models.Person.from_tag(match.group(1), None if match.group(2) == '' else int(match.group(2)))
        if person is None:
            # skip this tag but convert the remaining text recursively
            return f'{tags_to_mentions(text[:match.start()])}{match.group(0)}{tags_to_mentions(text[match.end():])}'
        else:
            text = f'{text[:match.start()]}<@{person.snowflake_or_wmbid}>{text[match.end():]}'
