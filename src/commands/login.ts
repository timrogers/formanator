import * as commander from 'commander';
import chalk from 'chalk';

import { actionRunner, prompt } from '../utils.js';
import { setAccessToken } from '../config.js';
import { exchangeIdAndTkForAccessToken, requestMagicLink } from '../forma.js';

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

const EMAIL_REGEX =
  /^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9-]+(?:\.[a-zA-Z0-9-]+)*$/;

const promptForEmail = (isFirstRun = true): string => {
  const promptMessage = isFirstRun
    ? 'Enter the email address you use to log on to Forma, then press Enter.'
    : chalk.yellow("That doesn't look like a valid email address. Please try again.");
  console.log(promptMessage);

  const email = prompt('> ');

  if (!EMAIL_REGEX.test(email)) {
    return promptForEmail(false);
  } else {
    return email;
  }
};

const promptForEmailedMagicLink = (
  email: string,
  errorMessage: string | null = null,
): { id: string; tk: string } => {
  const promptMessage = errorMessage
    ? chalk.yellow("That doesn't look like a valid magic link. Please try again.")
    : `Copy and paste the magic link sent to you at ${email}, then press Enter.`;
  console.log(promptMessage);

  const emailedMagicLink = prompt('> ');

  try {
    return parseEmailedFormaMagicLink(emailedMagicLink);
  } catch (e) {
    return promptForEmailedMagicLink(email, e);
  }
};

command
  .name('login')
  .description('Connect Formanator to your Forma account with a magic link')
  .option('--email <email>', 'Email address used to log in to Forma')
  .action(
    actionRunner(async (opts: Arguments) => {
      const email = opts.email ?? promptForEmail();
      await requestMagicLink(email);

      const { id, tk } = promptForEmailedMagicLink(email);
      const accessToken = await exchangeIdAndTkForAccessToken(id, tk);
      setAccessToken(accessToken);

      console.log(chalk.green('You are now logged in! 🥳'));
    }),
  );

export default command;
