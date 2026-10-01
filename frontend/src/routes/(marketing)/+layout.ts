// Marketing pages render on the server and ship no Svelte client code; app.html's inline
// script handles the few interactive bits.
export const ssr = true;
export const csr = false;
