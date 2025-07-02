/**
 * thirty_papers.js
 *
 * Contains all the client-side logic for the "30 Papers in 30 Days" project page.
 * This script is loaded on-demand by the main WASM application and handles
 * loading its own dependencies.
 */

/**
 * A helper function to dynamically load a script dependency if needed.
 * @param {string} url - The URL of the script to load.
 * @returns {Promise<void>} A promise that resolves when the script is loaded.
 */
function loadDependency(url) {
    return new Promise((resolve, reject) => {
        // Avoid loading the same script twice
        if (document.querySelector(`script[src="${url}"]`)) {
            console.log(`Dependency already loaded: ${url}`);
            resolve();
            return;
        }
        const script = document.createElement('script');
        script.src = url;
        script.onload = () => {
            console.log(`Successfully loaded dependency: ${url}`);
            resolve();
        };
        script.onerror = () => {
            console.error(`Failed to load dependency: ${url}`);
            reject(new Error(`Script load error for ${url}`));
        };
        document.head.appendChild(script);
    });
}


/**
 * Fetches a paper's markdown content, converts it to HTML,
 * and wraps it in a collapsible <details> element.
 * @param {string} file - The filename of the markdown file.
 * @returns {Promise<HTMLElement|null>} A promise that resolves to the HTML element or null on error.
 */
async function renderPaper(file) {
    try {
        const response = await fetch(`/api/projects/thirty_papers/papers/${file}`, {cache: "no-store"});
        if (!response.ok) {
             console.error(`Could not load paper: ${file}`);
             return null;
        }
        const markdown = await response.text();

        // Use marked.js to convert markdown to HTML
        const html = marked.parse(markdown);

        // Create a temporary div to safely parse the generated HTML
        const tempDiv = document.createElement('div');
        tempDiv.innerHTML = html;
        
        // Extract title from the first H1 tag for the summary line
        const h1 = tempDiv.querySelector('h1');
        const title = h1 ? h1.textContent : file.replace('.md', '');
        if (h1) h1.remove(); // Remove the h1 from the content to avoid duplication

        // Create the collapsible element structure
        const details = document.createElement('details');
        details.className = 'paper-entry';
        const summary = document.createElement('summary');
        summary.textContent = title;
        const content = document.createElement('div');
        content.className = 'paper-content';
        content.innerHTML = tempDiv.innerHTML;
        details.appendChild(summary);
        details.appendChild(content);

        return details;
    } catch (error) {
        console.error('Error processing paper:', file, error);
        const errorElement = document.createElement('div');
        errorElement.className = 'paper-entry';
        errorElement.textContent = `Error loading: ${file}`;
        return errorElement;
    }
}

/**
 * Initializes the paper loading process. This function is attached
 * to the window object so it can be called by external scripts (like WASM).
 */
window.init30PapersPage = async () => {
    // Step 1: Ensure the 'marked' library dependency is loaded.
    try {
        if (typeof marked === 'undefined') {
            await loadDependency('https://cdn.jsdelivr.net/npm/marked/marked.min.js');
        }
    } catch (error) {
        console.error("Fatal: Could not load the 'marked' library. Aborting page initialization.", error);
        // Optionally, display an error message to the user on the page.
        const papersListContainer = document.getElementById('papers-list');
        if(papersListContainer) {
            papersListContainer.innerHTML = `<p style="color: red;">Error: Could not load required library to render this page.</p>`;
        }
        return;
    }

    // Step 2: Proceed with the rest of the page initialization.
    const papersListContainer = document.getElementById('papers-list');
    const loadingState = document.getElementById('loading-state');

    if (!papersListContainer || papersListContainer.hasAttribute('data-loaded')) {
        return;
    }
    papersListContainer.setAttribute('data-loaded', 'true');

    // --- Configuration ---
    // To add a new paper, just add its filename to this array.
    const papers = [
        "Computing Machinery and Intelligence.md",
    ];
    // ---------------------

    const fragment = document.createDocumentFragment();
    for (const file of papers) {
        const paperElement = await renderPaper(file);
        if (paperElement) {
            fragment.appendChild(paperElement);
        }
    }
    
    if (loadingState) {
        loadingState.remove();
    }
    papersListContainer.appendChild(fragment);
};
