import * as commander from 'commander';
import Table from 'cli-table';

import { actionRunner } from '../utils.js';
import { getAccessToken } from '../config.js';
import { getClaimsList } from '../forma.js';
import VERSION from '../version.js';

const command = new commander.Command();

interface Arguments {
  accessToken?: string;
  page: string;
}

command
  .name('list-claims')
  .version(VERSION)
  .description('List claims in your Forma account and their current status')
  .option('--access-token <access_token>', 'Access token used to authenticate with Forma')
  .option('-p, --page <page>', 'Page number to retrieve (default: 0)', '0')
  .action(
    actionRunner(async (opts: Arguments) => {
      const accessToken = opts.accessToken ?? getAccessToken();
      const page = parseInt(opts.page, 10);

      if (!accessToken) {
        throw new Error(
          "You aren't logged in to Forma. Please run `npx formanator login` first.",
        );
      }

      const claims = await getClaimsList(accessToken, page);

      const table = new Table({
        head: [
          'Reimbursement Vendor',
          'Employee Note',
          'Amount',
          'Category',
          'Subcategory',
          'Status',
          'Reimbursement Status',
          'Payout Status',
          'Date Processed',
          'Note',
        ],
      });

      for (const claim of claims) {
        table.push([
          `${claim.reimbursement_vendor}`,
          `${claim.employee_note}`,
          `${claim.amount}`,
          `${claim.category}`,
          `${claim.subcategory}`,
          `${claim.status}`,
          `${claim.reimbursement_status}`,
          `${claim.payout_status}`,
          `${claim.date_processed}`,
          `${claim.note}`,
        ]);
      }

      console.log(table.toString());
    }),
  );

export default command;
