const express = require("express");
const path = require("path");
const cors = require("cors");

const app = express();
const port = 3000;

// Use CORS middleware to allow all origins
app.use(cors());

// Define the directory where your file is located. Adjust as necessary.
const publicDirectoryPath = path.join(__dirname, "../data"); // Ensure this path correctly points to the 'data' folder
console.log("Serving static files from:", publicDirectoryPath);

// Serve files in the public directory as static resources
app.use(express.static(publicDirectoryPath));

app.listen(port, () => {
  console.log(`Server is up on http://localhost:${port}`);
});
