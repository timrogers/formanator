export interface ReceiptFile {
  id: string;
  filename: string;
  date: string;
  amount?: string;
  merchant?: string;
  description?: string;
  mimeType: string;
}

export interface ProviderAuth {
  cookies: Record<string, string>;
}

export interface IntegrationProvider {
  name: string;
  displayName: string;

  validateAuth(auth: ProviderAuth): Promise<boolean>;

  listReceipts(
    auth: ProviderAuth,
    options?: { startDate?: string; endDate?: string },
  ): Promise<ReceiptFile[]>;

  downloadReceipt(
    auth: ProviderAuth,
    receiptId: string,
  ): Promise<{ data: Buffer; mimeType: string; filename: string }>;
}
