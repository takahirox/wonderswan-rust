const openTimeoutMs = 2000;

function openDatabase(
  name: string, label: string, upgrade: (db: IDBDatabase) => void,
): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(name, 1);
    let expired = false;
    const fail = (error: unknown) => {
      expired = true;
      window.clearTimeout(timeout);
      reject(error);
    };
    // An open queued behind a blocked deletion may never receive a blocked event.
    const timeout = window.setTimeout(() => fail(new Error(`${label} storage timed out.`)), openTimeoutMs);
    request.onblocked = () => fail(new Error(`${label} storage is busy.`));
    request.onupgradeneeded = () => {
      if (expired) {
        request.transaction!.abort();
        return;
      }
      try {
        upgrade(request.result);
      } catch (error) {
        request.transaction!.abort();
        fail(error);
      }
    };
    request.onerror = () => fail(request.error);
    request.onsuccess = () => {
      window.clearTimeout(timeout);
      const db = request.result;
      if (expired) db.close();
      else {
        db.onversionchange = () => db.close();
        resolve(db);
      }
    };
  });
}

export function database(name: string, label: string, upgrade: (db: IDBDatabase) => void) {
  return async function transaction<T>(
    storeNames: string | string[], mode: IDBTransactionMode, run: (tx: IDBTransaction) => T,
  ): Promise<T> {
    const db = await openDatabase(name, label, upgrade);
    try {
      return await new Promise<T>((resolve, reject) => {
        const tx = db.transaction(storeNames, mode);
        tx.onabort = () => reject(tx.error ?? new Error(`${label} storage transaction aborted.`));
        try {
          const requests = run(tx);
          tx.oncomplete = () => resolve(requests);
        } catch (error) {
          tx.abort();
          reject(error);
        }
      });
    } finally {
      db.close();
    }
  };
}
