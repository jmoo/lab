const vscode = require('vscode');
const child_process = require('child_process')
const config = require('./package.json')

function registerCommand(command, context) {
    console.log(`Registering command: ${command.id}`)
    context.subscriptions.push(vscode.commands.registerCommand(command.id, function () {
        console.log(`Command triggered: ${command.id}`)
        if (command.exec != null) {
            console.log(`Executing child process for ${command.id}: ${command.exec}`)
            child_process.execSync(command.exec), { stdio: 'inherit' }
        }

        if (command.require != null) {
            console.log(`Executing js for ${command.id}: ${command.require}`)
            require(command.require)
        }

        for (cmd in command.commands) {
            console.log(`Executing extension command for ${command.id}: ${command.commands[cmd]}`)
            vscode.commands.executeCommand(command.commands[cmd]);
        }
    }));
}

/**
 * @param {vscode.ExtensionContext} context
 */
function activate(context) {
    console.log(`"${config.name}" has been activated`);

    for (i in config.nixExtension.commands) {
        registerCommand(config.nixExtension.commands[i], context)
    }
}

// This method is called when your extension is deactivated
function deactivate() {
    console.log(`"${config.name}" is has been deactivated`);
}

module.exports = {
    activate,
    deactivate
}
