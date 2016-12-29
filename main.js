var child_process = require('child_process');

exports.handler = function(event, context, callback) {
    console.log(event);

    var git_url = event['body-json']['git-url'];
    var git_commit = event['body-json']['git-commit'];

    // Git commit is optional
    if (!!git_commit) {
	var args = [git_url, git_commit];
    } else {
	var args = [git_url];
    };

    // spawn a child process to run the binary
    var proc = child_process.spawn(
	'./target/release/harbor',
	args,
	// proc.stdin, pro.stdout, proc.stderr
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
