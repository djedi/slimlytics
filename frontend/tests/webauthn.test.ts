import { describe, expect, it } from 'vitest';
import { fromBase64Url, toBase64Url, creationOptions, requestOptions, serializeCredential } from '../src/lib/webauthn';

describe('webauthn encoding', () => {
  it('round-trips base64url without padding', () => {
    const bytes = new Uint8Array([0, 251, 255, 62, 63, 1]);
    const encoded = toBase64Url(bytes);
    expect(encoded).not.toMatch(/[+/=]/);
    expect(Array.from(fromBase64Url(encoded))).toEqual(Array.from(bytes));
  });

  it('decodes the binary fields of server creation options', () => {
    const options = creationOptions({
      publicKey: {
        challenge: toBase64Url(new Uint8Array([1, 2, 3])),
        rp: { id: 'localhost', name: 'Slimlytics' },
        user: { id: toBase64Url(new Uint8Array([9, 9])), name: 'a@example.com', displayName: 'a@example.com' },
        pubKeyCredParams: [{ type: 'public-key', alg: -7 }],
        excludeCredentials: [{ type: 'public-key', id: toBase64Url(new Uint8Array([7])) }]
      }
    });
    expect(Array.from(new Uint8Array(options.publicKey!.challenge as ArrayBuffer))).toEqual([1, 2, 3]);
    expect(Array.from(new Uint8Array(options.publicKey!.user.id as ArrayBuffer))).toEqual([9, 9]);
    expect(Array.from(new Uint8Array(options.publicKey!.excludeCredentials![0].id as ArrayBuffer))).toEqual([7]);
  });

  it('decodes request options and drops conditional mediation', () => {
    const options = requestOptions({
      publicKey: { challenge: toBase64Url(new Uint8Array([4])), allowCredentials: [{ type: 'public-key', id: toBase64Url(new Uint8Array([5])) }] },
      mediation: 'conditional'
    });
    expect('mediation' in options).toBe(false);
    expect(Array.from(new Uint8Array(options.publicKey!.allowCredentials![0].id as ArrayBuffer))).toEqual([5]);
  });

  it('serializes assertions with base64url fields for the server', () => {
    const buffer = (values: number[]) => new Uint8Array(values).buffer;
    const credential = {
      id: 'abc',
      type: 'public-key',
      rawId: buffer([1]),
      response: { clientDataJSON: buffer([2]), authenticatorData: buffer([3]), signature: buffer([4]), userHandle: buffer([5]) },
      getClientExtensionResults: () => ({})
    } as unknown as PublicKeyCredential;
    expect(serializeCredential(credential)).toEqual({
      id: 'abc',
      type: 'public-key',
      rawId: 'AQ',
      response: { clientDataJSON: 'Ag', authenticatorData: 'Aw', signature: 'BA', userHandle: 'BQ' },
      extensions: {}
    });
  });
});
