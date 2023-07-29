import * as commander from 'commander';
import Table from 'cli-table';
import { actionRunner } from '../utils.js';
import { getAccessToken } from '../config.js';
import { getBenefits } from '../forma.js';
const command = new commander.Command();
command
    .name('benefits')
    .description('List benefits in your Forma account and their remaining balances')
    .option('--access_token <access_token>', 'Access token used to authenticate with Forma')
    .action(actionRunner(async (opts) => {
    const accessToken = opts.accessToken ?? getAccessToken();
    if (!accessToken) {
        throw new Error("You aren't logged in to Forma. Please run `formanator login` first.");
    }
    const benefits = await getBenefits(accessToken);
    const table = new Table({
        head: ['Name', 'Remaining Amount'],
    });
    for (const benefit of benefits) {
        table.push([
            benefit.name,
            `${benefit.remainingAmount} ${benefit.remainingAmountCurrency}`,
        ]);
    }
    console.log(table.toString());
}));
export default command;
