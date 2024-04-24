async function sha256(message) {
    // Encode the string into bytes
    const msgBuffer = new TextEncoder().encode(message);

    // Hash the message
    const hashBuffer = await crypto.subtle.digest('SHA-256', msgBuffer);

    // Convert the ArrayBuffer to hex string
    const hashArray = Array.from(new Uint8Array(hashBuffer));
    return hashArray.map(b => b.toString(16).padStart(2, '0')).join('');
}

async function login() {
    const username = document.getElementById('loginUsername').value;
    const password = document.getElementById('loginPassword').value;
    const hash = await sha256(password);

    const response = await fetch('/auth', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            'Access-Control-Allow-Origin': "*",
        },
        body: JSON.stringify({username: username, password: hash})
    });

    const data = await response.json();

    if (response.ok) {
        localStorage.setItem('token', data.success.token);
        window.location.href = '/dashboard'; // todo fixme
    } else {
        alert('Login failed with error: ' + data.error.message);
    }
}

async function signup(signupUsername, signupPassword, adminId, adminPassword, name, authoruty, batch) {
    const signupHash = await sha256(signupPassword);
    const adminHash = await sha256(adminPassword);
    let batchInfo = null;
    if (authoruty === 2) {
        batchInfo = batch
    }

    let req = {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            'Access-Control-Allow-Origin': "*",
        },
        body: JSON.stringify({
            username: signupUsername,
            password: signupHash,
            signup_details: {
                name: name,
                admin_username: adminId,
                admin_password: adminHash,
                authority: authoruty,
                batch: batchInfo
            }
        })
    };
    console.log(JSON.stringify(req))
    const response = await fetch('/auth', req);
    console.log(response)

    const data = await response.json();

    if (response.ok) {
        localStorage.setItem('token', data.success.token);
        window.location.href = '/dashboard'; // todo fixme
    } else {
        alert('Login failed with error: ' + data.error.message);
    }
}