import { type IntegrationProvider } from './provider.js';
import { UberProvider } from './uber/provider.js';

const providers: Map<string, IntegrationProvider> = new Map();

export const registerProvider = (provider: IntegrationProvider): void => {
  providers.set(provider.name, provider);
};

export const getProvider = (name: string): IntegrationProvider | undefined => {
  return providers.get(name);
};

export const listProviders = (): IntegrationProvider[] => {
  return Array.from(providers.values());
};

// Register built-in providers
registerProvider(new UberProvider());
