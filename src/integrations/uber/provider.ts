import {
  type IntegrationProvider,
  type ProviderAuth,
  type ReceiptFile,
} from '../provider.js';

let playwright: typeof import('playwright') | null = null;

const loadPlaywright = async () => {
  if (playwright) return playwright;
  try {
    playwright = await import('playwright');
    return playwright;
  } catch {
    throw new Error(
      'Playwright is required for the Uber integration. Install it with: npm install playwright && npx playwright install chromium',
    );
  }
};

const UBER_TRIPS_URL = 'https://riders.uber.com/trips';

interface UberTrip {
  id: string;
  date: string;
  amount: string;
  description: string;
}

export class UberProvider implements IntegrationProvider {
  name = 'uber';
  displayName = 'Uber';

  async validateAuth(auth: ProviderAuth): Promise<boolean> {
    const pw = await loadPlaywright();
    const browser = await pw.chromium.launch({ headless: true });

    try {
      const context = await browser.newContext();
      await context.addCookies(
        Object.entries(auth.cookies).map(([name, value]) => ({
          name,
          value,
          domain: '.uber.com',
          path: '/',
        })),
      );

      const page = await context.newPage();
      await page.goto(UBER_TRIPS_URL, { waitUntil: 'networkidle' });

      // If we're redirected to a login page, cookies are invalid
      const url = page.url();
      const isValid = url.includes('riders.uber.com/trips');

      await context.close();
      return isValid;
    } catch {
      return false;
    } finally {
      await browser.close();
    }
  }

  async listReceipts(
    auth: ProviderAuth,
    options?: { startDate?: string; endDate?: string },
  ): Promise<ReceiptFile[]> {
    const pw = await loadPlaywright();
    const browser = await pw.chromium.launch({ headless: true });

    try {
      const context = await browser.newContext();
      await context.addCookies(
        Object.entries(auth.cookies).map(([name, value]) => ({
          name,
          value,
          domain: '.uber.com',
          path: '/',
        })),
      );

      const page = await context.newPage();
      await page.goto(UBER_TRIPS_URL, { waitUntil: 'networkidle' });

      // Scrape trip list from the page
      const trips: UberTrip[] = await page.evaluate(() => {
        const tripElements = document.querySelectorAll('[data-testid="trip-card"]');
        const results: { id: string; date: string; amount: string; description: string }[] = [];

        tripElements.forEach((el) => {
          // Extract trip data from the card elements
          // Uber's trip cards typically contain date, price, and route info
          const linkEl = el.querySelector('a');
          const href = linkEl?.getAttribute('href') ?? '';
          const idMatch = href.match(/trips\/([a-f0-9-]+)/);
          const id = idMatch?.[1] ?? '';

          const textContent = el.textContent ?? '';
          // Try to extract price (looks like $XX.XX or CA$XX.XX)
          const priceMatch = textContent.match(/(?:CA?\$|USD?\s?)(\d+\.\d{2})/);
          const amount = priceMatch?.[1] ?? '';

          // Try to extract date
          const dateEl = el.querySelector('time');
          const date = dateEl?.getAttribute('datetime') ?? '';

          // Use remaining text as description
          const description = textContent.replace(/\s+/g, ' ').trim();

          if (id) {
            results.push({ id, date, amount, description });
          }
        });

        return results;
      });

      await context.close();

      // Filter by date range if specified
      let filtered = trips;
      if (options?.startDate) {
        filtered = filtered.filter((t) => t.date >= options.startDate!);
      }
      if (options?.endDate) {
        filtered = filtered.filter((t) => t.date <= options.endDate!);
      }

      return filtered.map((trip) => ({
        id: trip.id,
        filename: `uber-trip-${trip.date || trip.id}.pdf`,
        date: trip.date,
        amount: trip.amount || undefined,
        merchant: 'Uber',
        description: trip.description,
        mimeType: 'image/png',
      }));
    } finally {
      await browser.close();
    }
  }

  async downloadReceipt(
    auth: ProviderAuth,
    receiptId: string,
  ): Promise<{ data: Buffer; mimeType: string; filename: string }> {
    const pw = await loadPlaywright();
    const browser = await pw.chromium.launch({ headless: true });

    try {
      const context = await browser.newContext();
      await context.addCookies(
        Object.entries(auth.cookies).map(([name, value]) => ({
          name,
          value,
          domain: '.uber.com',
          path: '/',
        })),
      );

      const page = await context.newPage();
      const tripUrl = `https://riders.uber.com/trips/${receiptId}`;
      await page.goto(tripUrl, { waitUntil: 'networkidle' });

      // Take a screenshot of the receipt page as the receipt image
      const screenshot = await page.screenshot({ fullPage: true, type: 'png' });

      await context.close();

      return {
        data: Buffer.from(screenshot),
        mimeType: 'image/png',
        filename: `uber-receipt-${receiptId}.png`,
      };
    } finally {
      await browser.close();
    }
  }
}
