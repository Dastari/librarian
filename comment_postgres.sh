#!/bin/bash

# Script to comment out PostgreSQL-specific code blocks in Rust files
# This keeps the code as reference but ensures only SQLite implementations are active

echo "Commenting out PostgreSQL code blocks..."

# Find all Rust files with PostgreSQL feature flags
find backend/src -name "*.rs" -exec grep -l '#\[cfg(feature = "postgres"\)\]' {} \; | while read file; do
    echo "Processing $file..."

    # Use sed to comment out PostgreSQL blocks
    # This is a complex pattern, so we'll do it in steps

    # First, comment out #[cfg(feature = "postgres")] lines and the following block
    sed -i '/#\[cfg(feature = "postgres"\)\]/,/^    }$/{
        /#\[cfg(feature = "postgres"\)\]/{
            i\
    // NOTE: PostgreSQL implementation commented out - keeping for reference
            s/.*/\/\/ &/
        }
        /^    }$/!{
            /^#\[cfg(feature = "postgres"\)\]/!s/.*/\/\/ &/
        }
    }' "$file"

    # Also comment out any standalone NOW() calls that aren't in feature blocks
    sed -i 's/NOW()/datetime('\''now'\'')/g' "$file"

    # Comment out TIMESTAMPTZ references
    sed -i 's/TIMESTAMPTZ/TEXT/g' "$file"
done

echo "Done commenting out PostgreSQL code blocks."