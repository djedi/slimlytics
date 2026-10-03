// Bridges the server's JSON WebAuthn options (binary fields as base64url) and the browser's
// navigator.credentials API (ArrayBuffers), in both directions.

type Json = Record<string, unknown>;

export function toBase64Url(value: ArrayBuffer | Uint8Array): string {
  const bytes = value instanceof Uint8Array ? value : new Uint8Array(value);
  let binary = '';
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

export function fromBase64Url(value: string): Uint8Array<ArrayBuffer> {
  const base64 = value.replace(/-/g, '+').replace(/_/g, '/');
  const binary = atob(base64 + '='.repeat((4 - (base64.length % 4)) % 4));
  const bytes = new Uint8Array(new ArrayBuffer(binary.length));
  for (let index = 0; index < binary.length; index += 1) bytes[index] = binary.charCodeAt(index);
  return bytes;
}

const buffer = (value: unknown) => fromBase64Url(String(value)).buffer;

function credentialList(list: unknown) {
  return Array.isArray(list)
    ? list.map((item: Json) => ({ ...item, id: buffer(item.id) }) as PublicKeyCredentialDescriptor)
    : undefined;
}

export function creationOptions(server: Json): CredentialCreationOptions {
  const publicKey = server.publicKey as Json;
  const user = publicKey.user as Json;
  return {
    publicKey: {
      ...publicKey,
      challenge: buffer(publicKey.challenge),
      user: { ...user, id: buffer(user.id) },
      excludeCredentials: credentialList(publicKey.excludeCredentials)
    } as PublicKeyCredentialCreationOptions
  };
}

/** The server may suggest conditional (autofill) mediation; sign-in here is button-driven. */
export function requestOptions(server: Json): CredentialRequestOptions {
  const publicKey = server.publicKey as Json;
  return {
    publicKey: {
      ...publicKey,
      challenge: buffer(publicKey.challenge),
      allowCredentials: credentialList(publicKey.allowCredentials)
    } as PublicKeyCredentialRequestOptions
  };
}

/** Converts a browser credential into the JSON shape the server verifies. */
export function serializeCredential(credential: PublicKeyCredential) {
  const response: Record<string, string | null> = {};
  for (const key of ['clientDataJSON', 'attestationObject', 'authenticatorData', 'signature', 'userHandle'] as const) {
    const value = (credential.response as unknown as Record<string, ArrayBuffer | null | undefined>)[key];
    if (value) response[key] = toBase64Url(value);
  }
  return {
    id: credential.id,
    type: credential.type,
    rawId: toBase64Url(credential.rawId),
    response,
    extensions: credential.getClientExtensionResults?.() ?? {}
  };
}

export function passkeysSupported(): boolean {
  return typeof window !== 'undefined' && typeof window.PublicKeyCredential === 'function' && !!navigator.credentials;
}

/** Turns browser WebAuthn failures into messages people can act on. */
export function passkeyErrorMessage(reason: unknown): string {
  if (reason instanceof DOMException) {
    if (reason.name === 'NotAllowedError') return 'The passkey request was cancelled or timed out.';
    if (reason.name === 'InvalidStateError') return 'This device already has a passkey for your account.';
    if (reason.name === 'SecurityError') return 'Passkeys need this site to be opened at its public address over HTTPS.';
  }
  return reason instanceof Error ? reason.message : 'The passkey could not be used.';
}

export async function createPasskey(serverOptions: Json) {
  const credential = (await navigator.credentials.create(creationOptions(serverOptions))) as PublicKeyCredential | null;
  if (!credential) throw new Error('No passkey was created.');
  return serializeCredential(credential);
}

export async function getPasskey(serverOptions: Json) {
  const credential = (await navigator.credentials.get(requestOptions(serverOptions))) as PublicKeyCredential | null;
  if (!credential) throw new Error('No passkey was selected.');
  return serializeCredential(credential);
}
