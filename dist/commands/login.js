import * as commander from 'commander';
import { actionRunner, prompt } from '../utils.js';
import { setAccessToken } from '../config.js';
const command = new commander.Command();
const requestMagicLink = async (email) => {
    const response = await fetch('https://api.joinforma.com/client/auth/v2/login/magic?is_mobile=true', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
        body: JSON.stringify({ email }),
    });
    if (!response.ok) {
        throw new Error('Unable to request magic link');
    }
    const parsedResponse = (await response.json());
    if (!parsedResponse.success) {
        throw new Error('Unable to request magic link');
    }
};
const isEmailedFormaMagicLink = (emailedMagicLink) => emailedMagicLink.hostname === 'joinforma.page.link' &&
    emailedMagicLink.protocol === 'https:' &&
    emailedMagicLink.pathname === '/' &&
    emailedMagicLink.searchParams.has('link');
const exchangeEmailedMagicLinkForToken = async (emailedMagicLink) => {
    const parsedEmailedMagicLink = new URL(emailedMagicLink);
    if (!isEmailedFormaMagicLink(parsedEmailedMagicLink)) {
        throw new Error("The provided link doesn't look like a real Forma magic link.");
    }
    const urlEncodedMagicLink = parsedEmailedMagicLink.searchParams.get('link');
    const realMagicLinkAsString = decodeURIComponent(urlEncodedMagicLink);
    const realMagicLink = new URL(realMagicLinkAsString);
    const idFromMagicLink = realMagicLink.searchParams.get('id');
    const tkFromMagicLink = realMagicLink.searchParams.get('tk');
    if (!idFromMagicLink || !tkFromMagicLink) {
        throw new Error("The provided link doesn't look like a real Forma magic link.");
    }
    const requestUrl = new URL('https://api.joinforma.com/client/auth/v2/login/magic');
    requestUrl.search = new URLSearchParams({
        id: idFromMagicLink,
        tk: tkFromMagicLink,
        return_token: 'true',
        is_mobile: 'true',
    }).toString();
    const response = await fetch(requestUrl);
    if (!response.ok) {
        throw new Error(`Something went wrong when exchanging the magic link for a token - expected \`200 OK\` response, got \`${response.status} ${response.statusText}\`.`);
    }
    const parsedResponse = (await response.json());
    if (!parsedResponse.success) {
        throw new Error('Something went wrong when exchanging the magic link for a token. Received a `200 OK` response, but the response body indicated that the request was not successful');
    }
    return parsedResponse.data.auth_token;
};
command
    .name('login')
    .description('Connect Formanator to your Forma account with a magic link')
    .option('--email <email>', 'Email address used to log in to Forma')
    .option('--magic-link-url <magic_link_url>', 'Magic link received by email for logging in to Forma')
    .action(actionRunner(async (opts) => {
    if (opts.email && opts.magicLinkUrl) {
        throw new Error('You must provide either --email or --magic-link-url, not both');
    }
    if (opts.magicLinkUrl) {
        const accessToken = await exchangeEmailedMagicLinkForToken(opts.magicLinkUrl);
        setAccessToken(accessToken);
    }
    else if (opts.email) {
        await requestMagicLink(opts.email);
        console.log(`Copy and paste the magic link sent to you at ${opts.email}, then press Enter.`);
        const magicLink = prompt('> ');
        const accessToken = await exchangeEmailedMagicLinkForToken(magicLink);
        setAccessToken(accessToken);
    }
    else {
        console.log('Enter the email address you use to log on to Forma, then press Enter.');
        const email = prompt('> ');
        await requestMagicLink(email);
        console.log(`Copy and paste the magic link sent to you at ${email}, then press Enter.`);
        const magicLink = prompt('> ');
        const accessToken = await exchangeEmailedMagicLinkForToken(magicLink);
        setAccessToken(accessToken);
    }
    console.log('You are now logged in! 🥳');
}));
export default command;
