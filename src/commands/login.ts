import * as commander from 'commander';
import chalk from 'chalk';
import open from 'open';

import { actionRunner, prompt } from '../utils.js';
import { storeConfig, getEmail } from '../config.js';
import { exchangeIdAndTkForAccessToken } from '../forma.js';
import VERSION from '../version.js';

const command = new commander.Command();

interface Arguments {
  email?: string;
}

const parseEmailedFormaMagicLink = (input: string): { id: string; tk: string } => {
  const parsedUrl = new URL(input);

  if (parsedUrl.hostname !== 'joinforma.page.link') {
    throw new Error(
      'Forma magic links are expected to have the hostname `joinforma.page.link`.',
    );
  }

  if (parsedUrl.protocol !== 'https:') {
    throw new Error('Forma magic links are expected to have the protocol `https:`.');
  }

  if (!parsedUrl.pathname.startsWith('/')) {
    throw new Error('Forma magic links are expected to not have a path.');
  }

  const magicLinkEmbeddedInUrl = parsedUrl.searchParams.get('link');

  if (!magicLinkEmbeddedInUrl) {
    throw new Error('Forma magic links are expected to have a `link` query parameter.');
  }

  const realMagicLinkAsString = decodeURIComponent(magicLinkEmbeddedInUrl);
  const realMagicLink = new URL(realMagicLinkAsString);

  const id = realMagicLink.searchParams.get('id');
  const tk = realMagicLink.searchParams.get('tk');

  if (!id || !tk) {
    throw new Error(
      'Forma magic links are expected to have a `link` query parameter containing a URL with an `id` and `tk` query parameter embedded inside.',
    );
  }

  return { id, tk };
};

const promptForEmailedMagicLink = (
  errorMessage: string | null = null,
): { id: string; tk: string } => {
  const promptMessage = errorMessage
    ? chalk.yellow("That doesn't look like a valid magic link. Please try again.")
    : 'Copy and paste the magic link from your email, then press Enter.';
  console.log(promptMessage);

  const emailedMagicLink = prompt('> ');

  try {
    return parseEmailedFormaMagicLink(emailedMagicLink);
  } catch (e) {
    return promptForEmailedMagicLink(e);
  }
};

command
  .name('login')
  .version(VERSION)
  .description(
    'Connect Formanator to your Forma account with a magic link. Your email address will be remembered after logging in for the first time.',
  )
  .option(
    '--email <email>',
    'The email address to use to log in to Forma. Defaults to the FORMA_EMAIL environment variable, or the email you last used to log in.',
    process.env.FORMA_EMAIL || getEmail(),
  )
  .action(
    actionRunner(async (opts: Arguments) => {
      console.log(
        chalk.blue(
          '\nA browser window will now open to the Forma login page.\n' +
            'Please enter your email address and request a magic link.\n' +
            'Once you receive the magic link in your email, come back here to paste it.\n',
        ),
      );

      await open('https://client.joinforma.com/login?type=magic');

      const { id, tk } = promptForEmailedMagicLink();
      const accessToken = await exchangeIdAndTkForAccessToken(id, tk);
      storeConfig({ accessToken, email: opts.email });

      console.log(chalk.green('You are now logged in! 🥳'));
    }),
  );

export default command;
