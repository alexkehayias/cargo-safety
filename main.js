var child_process = require('child_process');

exports.handler = function(event, context, callback) {
    console.log(event);

    // spawn a child process to run the binary
    var proc = child_process.spawn(
	'./target/release/harbor',
	[event["git-url"]],
	// proc.stdin, pro.stdout, proc.stderr]
	{stdio: ['ignore', 'pipe', 'ignore']}
    );

    var out = '';
    proc.stdout.on('data', function(data) {
         out += data.toString();
    });

    proc.on('close', function(code) {
	if(code !== 0) {
	    err = new Error("Process exited with non-zero status code");
	    return callback(err, null);
	}

	return callback(null, out);
    });
}
