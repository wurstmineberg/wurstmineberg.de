import flask
import flask_wtf
import markdown
import markdown.inlinepatterns
import markdown.util
import pathlib
import re

import wurstmineberg_web.models
import wurstmineberg_web.util

DISCORD_MENTION_REGEX = '<@!?({}|[0-9]+)>'.format(wurstmineberg_web.models.WMBID_REGEX)
DISCORD_TAG_REGEX = '@([^#]{2,32})#([0-9]{4}?)'

WIKI_ROOT = wurstmineberg_web.util.BASE_PATH / 'wiki'

class DiscordMentionPattern(markdown.inlinepatterns.LinkInlineProcessor):
    def handleMatch(self, m, data):
        person = wurstmineberg_web.models.Person.from_snowflake_or_wmbid(m.group(1))
        el = markdown.util.etree.Element('a')
        el.text = '@{}'.format(person.display_name)
        el.set('href', flask.url_for('profile', person=str(person.snowflake_or_wmbid)))
        return el, m.start(0), m.end(0)

class DiscordMentionExtension(markdown.Extension):
    def extendMarkdown(self, md, md_globals):
        config = self.getConfigs()
        md.inlinePatterns.add('discord-mention', DiscordMentionPattern(DISCORD_MENTION_REGEX, md), '<reference')

def mentions_to_tags(text):
    while True:
        match = re.search(DISCORD_MENTION_REGEX, text)
        if not match:
            return text
        person = wurstmineberg_web.models.Person.from_snowflake_or_wmbid(match.group(1))
        if person.discorddata is None:
            tag = '@{}#'.format(person.wmbid)
        else:
            tag = '@{}#{:04}'.format(person.discorddata['username'], person.discorddata['discriminator'])
        text = '{}{}{}'.format(text[:match.start()], tag, text[match.end():])

def tags_to_mentions(text):
    while True:
        match = re.search(DISCORD_TAG_REGEX, text)
        if not match:
            return text
        person = wurstmineberg_web.models.Person.from_tag(match.group(1), None if match.group(2) == '' else int(match.group(2)))
        text = '{}<@{}>{}'.format(text[:match.start()], person.snowflake_or_wmbid, text[match.end():])
