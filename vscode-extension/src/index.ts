import * as path from 'node:path';
import { workspace, ExtensionContext } from 'vscode';

import {
	LanguageClient,
	LanguageClientOptions,
	ServerOptions,
	TransportKind
} from 'vscode-languageclient/node';

let client: LanguageClient | null = null;

export function activate(context: ExtensionContext) {
	const serverOptions: ServerOptions = {
		run: {
			command: context.asAbsolutePath(path.join('lib', 'lsp-server-debug')),
			transport: TransportKind.stdio
		},
		debug: {
			command: context.asAbsolutePath(path.join('lib', 'lsp-server-debug')),
			transport: TransportKind.stdio
		}
	};

	let clientOptions: LanguageClientOptions = {
		documentSelector: [{ scheme: 'file', language: 'plaintext' }],
		synchronize: {
			fileEvents: workspace.createFileSystemWatcher('**/.clientrc')
		}
	};

	client = new LanguageClient(
		'pcrlLanguageServer',
		'PCRL language server',
		serverOptions,
		clientOptions
	);

	client.start();
}

export async function deactivate() {
	await client?.stop();
}
