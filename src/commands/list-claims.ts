import * as commander from 'commander';
import Table from 'cli-table';

import { actionRunner } from '../utils.js';
import { getAccessToken } from '../config.js';
import { getClaimsList } from '../forma.js';
import VERSION from '../version.js';

const command = new commander.Command();

interface Arguments {
  accessToken?: string;
  filter?: string;
}

command
  .name('list-claims')
  .version(VERSION)
  .description('List claims in your Forma account and their current status')
  .option('--access-token <access_token>', 'Access token used to authenticate with Forma')
  .option(
    '--filter <filter>',
    'Filter claims by status (currently supports: in_progress)',
  )
  .action(
    actionRunner(async (opts: Arguments) => {
      const accessToken = opts.accessToken ?? getAccessToken();

      if (!accessToken) {
        throw new Error(
          "You aren't logged in to Forma. Please run `npx formanator login` first.",
        );
      }

      if (opts.filter && opts.filter !== 'in_progress') {
        throw new Error(
          `Invalid filter value '${opts.filter}'. Currently supported filters: in_progress`,
        );
      }

      const claims = await getClaimsList(
        accessToken,
        opts.filter === 'in_progress' ? 'in_progress' : undefined,
      );

      // Check if any claims have non-null payout_status
      const hasPayoutStatus = claims.some((claim) => claim.payout_status !== null);

      const tableHeaders = [
        'Reimbursement Vendor',
        'Employee Note',
        'Amount',
        'Category',
        'Subcategory',
        'Status',
        'Reimbursement Status',
        ...(hasPayoutStatus ? ['Payout Status'] : []),
        'Date Processed',
        'Note',
      ];

      const table = new Table({
        head: tableHeaders,
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
          ...(hasPayoutStatus ? [`${claim.payout_status}`] : []),
          `${claim.date_processed}`,
          `${claim.note}`,
        ]);
      }

      console.log(table.toString());
    }),
  );

export default command;
