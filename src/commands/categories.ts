import * as commander from 'commander';
import Table from 'cli-table';

import { actionRunner } from '../utils.js';
import { getAccessToken } from '../config.js';
import { getCategoriesForBenefitName } from '../forma.js';
import VERSION from '../version.js';

const command = new commander.Command();

interface Arguments {
  accessToken?: string;
  benefit: string;
}

command
  .name('categories')
  .version(VERSION)
  .description('List categories available for a Forma benefit')
  .requiredOption('--benefit <benefit>', 'The benefit to list categories for')
  .option('--access-token <access_token>', 'Access token used to authenticate with Forma')
  .action(
    actionRunner(async (opts: Arguments) => {
      const accessToken = opts.accessToken ?? getAccessToken();

      if (!accessToken) {
        throw new Error(
          "You aren't logged in to Forma. Please run `npx formanator login` first.",
        );
      }

      const categories = await getCategoriesForBenefitName(accessToken, opts.benefit);

      const table = new Table({
        head: ['Parent Category', 'Category'],
      });

      for (const category of categories) {
        table.push([
          category.category_name,
          category.subcategory_alias ?? category.subcategory_name,
        ]);
      }

      console.log(table.toString());
    }),
  );

export default command;
