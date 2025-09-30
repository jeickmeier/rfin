import React from 'react';
import { Link } from 'react-router-dom';

const NotFound: React.FC = () => (
  <div className="not-found">
    <h1>Page Not Found</h1>
    <p className="page-lead">The example you are looking for does not exist.</p>
    <Link to="/" className="not-found__link">
      Return to all examples
    </Link>
  </div>
);

export default NotFound;
