import {
  LanguageClient,
} from 'vscode-languageclient';

let client: LanguageClient;

export function activate(_ctx: any) {
  client = new LanguageClient(
    'bangls',
    'Bang Language Server',
    { command: 'bangls', args: ['--vscode'] },
    { documentSelector: [{scheme: 'file', language: 'mdtlbl'}] }
  );
  client.start();
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}
