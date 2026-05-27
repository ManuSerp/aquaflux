#exemple scikit learn pipeline with 2 transformers and 1 estimator converted to aquaflux pipeline

from sklearn.pipeline import Pipeline
from sklearn.preprocessing import StandardScaler
from sklearn.decomposition import PCA
from sklearn.linear_model import LogisticRegression
import numpy as np
from sklearn.base import BaseEstimator, TransformerMixin

# Define pipeline
pipeline = Pipeline([
    ("scaler", StandardScaler()),        # Transformer 1
    ("pca", PCA(n_components=2)),        # Transformer 2
    ("clf", LogisticRegression())        # Final estimator
])





# ---- Custom Transformer 1 ----
class AddConstantFeature(BaseEstimator, TransformerMixin):
    def __init__(self, value=1.0):
        self.value = value

    def fit(self, X, y=None):
        return self  # nothing to learn

    def transform(self, X):
        constant_column = np.full((X.shape[0], 1), self.value)
        return np.hstack([X, constant_column])


# ---- Custom Transformer 2 ----
class MultiplyFeatures(BaseEstimator, TransformerMixin):
    def __init__(self, factor=2.0):
        self.factor = factor

    def fit(self, X, y=None):
        return self

    def transform(self, X):
        return X * self.factor




pipeline = Pipeline([
    ("add_const", AddConstantFeature(value=0.5)),  # Custom transformer 1
    ("multiply", MultiplyFeatures(factor=1.5)),   # Custom transformer 2
    ("clf", LogisticRegression(max_iter=200))     # Final estimator
])
