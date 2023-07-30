import * as commander from 'commander';
import chalk from 'chalk';
import { actionRunner } from '../utils.js';
import { getAccessToken } from '../config.js';
import { createClaim } from '../forma.js';
import { claimParamsToCreateClaimOptions } from '../claims.js';
const command = new commander.Command();
command
    .name('submit-claim')
    .description('Submit a claim for a Forma benefit')
    .requiredOption('--benefit <benefit>', 'The benefit you are claiming for')
    .requiredOption('--amount <amount>', 'The amount of the claim')
    .requiredOption('--merchant <merchant>', 'The name of the merchant')
    .requiredOption('--category <category>', 'The category of the claim')
    .requiredOption('--purchase-date <purchase-date>', 'The date of the purchase in YYYY-MM-DD format')
    .requiredOption('--description <description>', 'The description of the claim')
    .requiredOption('--receipt-path <receipt-path>', 'The path of the receipt. JPEG, PNG, PDF and HEIC files up to 10MB are accepted.')
    .option('--access-token <access_token>', 'Access token used to authenticate with Forma')
    .action(actionRunner(async (opts) => {
    const accessToken = opts.accessToken ?? getAccessToken();
    if (!accessToken) {
        throw new Error("You aren't logged in to Forma. Please run `formanator login` first.");
    }
    const createClaimOptions = await claimParamsToCreateClaimOptions(opts, accessToken);
    await createClaim(createClaimOptions);
    console.log(chalk.green('Claim submitted successfully ✅'));
}));
export default command;
